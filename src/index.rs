//! This module is only enabled for `#[cfg(any(feature = "alloc", feature = "std"))]`.
//!
//! This module provides advanced DevTree utilities which require an index over a flattened device
//! tree to remain performant. As such, we rely on an allocator to provide heap allocations to
//! generate and store this index.
//!
//! (Ideally we wouldn't require a full system allocator and could just use a small memory pool
//! allocator. If anyone knows of such an allocator/interface reach out and we might add this.)

#![allow(dead_code)] // TODO/FIXME
#![allow(unused_variables)]
#![allow(unused_imports)]
use crate::iters::AssociatedOffset;
use crate::unsafe_unwrap::UnsafeUnwrap;
use crate::*;

use core::marker::PhantomData;
use core::mem::{size_of, align_of};
use core::ptr::{null, null_mut};
use core::str;

unsafe fn ptr_in<T>(buf: &[u8], ptr: *const T) -> bool {
    buf.as_ptr().add(buf.len()) > (ptr as *const u8) &&
        buf.as_ptr() <= (ptr as *const u8)
}

unsafe fn aligned_ptr_in<T>(buf: &[u8], offset: usize) -> *mut T {
    let mut ptr = buf.as_ptr().add(offset);
    ptr = ptr.add(ptr.align_offset(align_of::<T>()));
    assert!(ptr_in(buf, ptr));
    ptr as *mut T
}

// TODO Add a wrapper around these that is easier to use (that includes a reference to the fdt).
pub struct DTIProp<'dt> {
    propbuf: &'dt [u8],
    nameoff: AssociatedOffset<'dt>,
}

impl<'dt> From<&iters::ParsedProp<'dt>> for DTIProp<'dt> {
    fn from(prop: &iters::ParsedProp<'dt>) -> Self {
        Self {
            propbuf: prop.prop_buf,
            nameoff: prop.name_offset,
        }
    }
}

// TODO Add a wrapper around these that is easier to use (that includes a reference to the fdt).
pub struct DevTreeIndex<'dt, 'i: 'dt> {
    fdt: &'i DevTree<'dt>,
    root: *mut DTINode<'dt, 'i>,
}

struct DevTreeIndexBuildState<'dt, 'i: 'dt> {
    buf: &'i mut [u8],
    cur_node: *mut DTINode<'dt, 'i>,
    last_new_node: *mut DTINode<'dt, 'i>,
    front_off: usize,
}

struct DevTreeIndexProp<'dt> {
    propbuf: &'dt [u8],
    nameoff: AssociatedOffset<'dt>,
}

struct UnsafeSlice<T> {
    ptr: *const T,
    size: usize,
}

struct DTINode<'dt, 'i: 'dt> {
    parent: *mut Self,
    first_child: *mut Self,
    /// `next` is either
    /// 1. the next sibling node
    /// 2. the next node in DFS (some higher up node)
    /// It is 1 if (*next).parent == self.parent, otherwise it is 2.
    next: *mut Self,
    name: &'dt [u8],
    // Note: we could save space on the pointer if we used a custom C-like array.
    // This would require repr(C) alignment, we'll just stick with rust alignment.
    //props: &'i[DevTreeIndexProp<'dt>],
    props: UnsafeSlice<DevTreeIndexProp<'dt>>,
    _index: PhantomData<&'i u8>,
}

impl<'dt, 'i:'dt> DevTreeIndexBuildState<'dt, 'i> {
    pub unsafe fn append_node(&mut self, node: &iters::ParsedBeginNode<'dt>) {

        // Align pointer
        let new_ptr = aligned_ptr_in::<DTINode>(self.buf, self.front_off);

        // Increment the next offset
        self.front_off = new_ptr.add(1) as usize - self.buf.as_ptr() as usize;

        // Get the address of this new nodes properties (stored directly after this)
        let prop_ptr = aligned_ptr_in::<DevTreeIndexProp>(self.buf, self.front_off);

        // Write the data
        *new_ptr = DTINode {
            parent: self.cur_node,
            // This is set by the next node we create.
            first_child: null_mut(),
            // This is set by the next node we create.
            next: null_mut(),
            name: node.name,
            props: UnsafeSlice {ptr: prop_ptr, size: 0},
            _index: PhantomData,
        };

        if !self.cur_node.is_null() {
            assert!(self.last_new_node != null_mut(),
                "cur_node should not be initialized without last_new_node");

            (*self.last_new_node).next = new_ptr;
            if let Some(prev_sibling) = (*self.cur_node).next.as_mut() {
                prev_sibling.next = new_ptr;
            }
            (*self.cur_node).next = new_ptr;

            // If this new node is the first node that follows the current one, it is the current's
            // first child.
            if (*self.cur_node).first_child.is_null() {
                (*self.cur_node).first_child = new_ptr;
            }
        }

        // Save the new node ptr.
        self.cur_node = new_ptr;
        self.last_new_node = new_ptr;
    }

    pub unsafe fn append_prop(&mut self, prop: &iters::ParsedProp<'dt>) {
        let props = &mut (*self.cur_node).props;
        props.size += 1;
        let new_ptr = props.ptr.add(props.size) as *mut DevTreeIndexProp<'dt>;
        *new_ptr = DevTreeIndexProp {
                propbuf: prop.prop_buf,
                nameoff: prop.name_offset,
        };
        self.front_off = new_ptr.add(1) as usize - self.buf.as_ptr() as usize;
    }
}

impl<'dt, 'i: 'dt> DevTreeIndex<'dt, 'i> {
    unsafe fn init_state(buf: &'i mut [u8], iter: &mut iters::DevTreeParseIter<'dt>) -> Result<DevTreeIndexBuildState<'dt, 'i>, DevTreeError> {
        let mut state = DevTreeIndexBuildState {
            front_off: 0,
            buf,
            cur_node: null_mut(),
            last_new_node: null_mut(),
        };

        for tok in iter {
            match tok {
                iters::ParsedTok::BeginNode(node) => {
                    state.append_node(&node);
                    return Ok(state);
                },
                iters::ParsedTok::Nop => continue,
                _ => return Err(DevTreeError::ParseError),
            }
        }
        Err(DevTreeError::ParseError)
    }

    // Note: This parsing method is particularly unsafe.
    //
    // We decide this is worth it:
    // - it requires no allocator
    // - it has incredibly low overhead
    // - it is *very* easy to test in isolation
    pub unsafe fn new(fdt: &'i DevTree<'dt>, buf: &'i mut [u8]) -> Result<Self, DevTreeError> {
        let mut iter = iters::DevTreeParseIter::new(fdt);

        let mut state = Self::init_state(buf, &mut iter)?;
        let this = Self {
            fdt,
            root: state.cur_node,
        };

        // Devtree Props may only occur before child nodes. We'll call this the "node_header".
        let mut in_node_header = true;

        // The buffer will be split into two parts, front and back:
        //
        // Front will be used as a temporary work section to  build the nodes as we parse them.
        // The back will be used to save completely parsed nodes.
        for tok in iter {
            match tok {
                iters::ParsedTok::BeginNode(node) => {
                    state.append_node(&node);
                    in_node_header = true;
                }
                iters::ParsedTok::Prop(prop) => {
                    if !in_node_header {
                        return Err(DevTreeError::ParseError);
                    }

                    // Push the prop
                    state.append_prop(&prop)
                }
                iters::ParsedTok::EndNode => {
                    in_node_header = false;

                    // There were more EndNode tokens than BeginNode ones.
                    if state.cur_node.is_null() {
                        return Err(DevTreeError::ParseError);
                    }

                    // Change the current node back to the parent.
                    state.cur_node = (*state.cur_node).parent;
                }
                iters::ParsedTok::Nop => continue,
            }
        }

        Ok(this)
    }

    pub fn dfs_iter<'a>(&'a self) -> DevTreeIndexNodeIter<'dt, 'i, 'a> {
        DevTreeIndexNodeIter::new(self)
    }
}

#[derive(Clone)]
pub struct DevTreeIndexNode<'dt, 'i: 'dt, 'a: 'i> {
    pub index: &'a DevTreeIndex<'dt, 'i>,
    node: &'a DTINode<'dt, 'i>,
}


impl<'dt, 'i: 'dt, 'a: 'i> DevTreeIndexNode<'dt, 'i, 'a> {
    fn new(node: &'a DTINode<'dt, 'i>, index: &'a DevTreeIndex<'dt, 'i>) -> Self {
        Self {
            node,
            index
        }
    }

    pub fn name(&self) -> &'dt str {
        str::from_utf8(self.node.name).unwrap()
    }
}

// TODO Note iterator
/// An interator over [`DevTreeNode`] objects in the [`DevTree`]
#[derive(Clone)]
pub struct DevTreeIndexNodeIter<'dt, 'i: 'dt, 'a: 'i> {
    index_node: Option<DevTreeIndexNode<'dt, 'i, 'a>>
}

impl<'dt, 'i: 'dt, 'a: 'i> DevTreeIndexNodeIter<'dt ,'i, 'a> {
    pub(crate) fn new(index: &'a DevTreeIndex<'dt, 'i>) -> Self {
        unsafe {
            let root_ref = index.root.as_ref().unsafe_unwrap();

            Self {
                index_node: Some(DevTreeIndexNode::new(root_ref, index))
            }
        }
    }

    // See the documentation of [`DevTree::find_node`]
    //#[inline]
    //pub fn find<F>(&mut self, predicate: F) -> Option<(DevTreeNode<'a>, Self)>
    //where
    //    F: Fn(&DevTreeNode) -> Result<bool, DevTreeError>,
    //{
    //}
}

impl<'dt, 'i: 'dt, 'a: 'i> Iterator for DevTreeIndexNodeIter<'dt ,'i, 'a> {
    type Item = DevTreeIndexNode<'dt ,'i, 'a>;
    fn next(&mut self) -> Option<Self::Item> {
        // DFS Iteration
        if let Some(idx) = &mut self.index_node {
            let cur = idx.clone();
            unsafe {
                if let Some(next) = idx.node.first_child.as_ref() {
                    println!("First_child");
                    idx.node = next;
                }
                else if let Some(next) = idx.node.next.as_ref() {
                    println!("next_node");
                    idx.node = next;
                }
                else {
                    self.index_node = None;
                }
            }
            return Some(cur);
        }
        None
    }
}

// TODO Fuck load of utility methods.

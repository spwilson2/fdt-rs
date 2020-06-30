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
use core::mem::{align_of, size_of};
use core::ptr::{null, null_mut};
use core::str;
use core::alloc::Layout;

unsafe fn ptr_in<T>(buf: &[u8], ptr: *const T) -> bool {
    // Make sure we dont' go over the buffer
    let mut res = buf.as_ptr() as usize + buf.len() > (ptr as usize + size_of::<T>());
    // Make sure we don't go under the buffer
    res &= buf.as_ptr() as usize <= ptr as usize;
    return res;
}

unsafe fn aligned_ptr_in<T>(buf: &[u8], offset: usize) -> Result<*mut T, DevTreeError> {
    let ptr = buf.as_ptr().add(offset);

    let ptr = ptr.add(ptr.align_offset(align_of::<T>())) as *mut T;
    if !ptr_in(buf, ptr) {
        return Err(DevTreeError::NotEnoughMemory);
    }
    Ok(ptr)
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
#[derive(Debug)]
pub struct DevTreeIndex<'dt, 'i: 'dt> {
    fdt: &'i DevTree<'dt>,
    root: *mut DTINode<'dt, 'i>,
}

struct DTIBuilder<'dt, 'i: 'dt> {
    buf: &'i mut [u8],
    cur_node: *mut DTINode<'dt, 'i>,
    prev_new_node: *mut DTINode<'dt, 'i>,
    front_off: usize,

    /// Devtree Props may only occur before child nodes.
    /// We'll call this the "node_header".
    in_node_header: bool,
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

    //NOTE: We store props like C arrays.
    // This the number of props after this node in memory.
    // Props are a packed array after each node.
    num_props: usize,
    _index: PhantomData<&'i u8>,
}

impl<'dt, 'i: 'dt> DTIBuilder<'dt, 'i> {
    fn allocate_aligned_ptr<T>(&mut self) -> Result<*mut T, DevTreeError> {
        unsafe {
            let ptr = aligned_ptr_in::<T>(self.buf, self.front_off)?;
            self.front_off = ptr.add(1) as usize - self.buf.as_ptr() as usize;
            Ok(ptr)
        }
    }

    pub fn parsed_node(&mut self, node: &iters::ParsedBeginNode<'dt>) -> Result<(), DevTreeError> {
        unsafe {
            self.in_node_header = true;

            let new_ptr = self.allocate_aligned_ptr::<DTINode>()?;
            let parent = self.cur_node;

            // Write the data
            *new_ptr = DTINode {
                parent,

                // set by the next node we create
                first_child: null_mut(),
                // set by the next node we create
                next: null_mut(),

                name: node.name,
                num_props: 0,
                _index: PhantomData,
            };

            if !parent.is_null() {
                debug_assert!(
                    self.prev_new_node != null_mut(),
                    "cur_node should not have been initialized without also intializing \
                    prev_new_node"
                );

                (*self.prev_new_node).next = new_ptr;
                if let Some(prev_sibling) = (*parent).next.as_mut() {
                    prev_sibling.next = new_ptr;
                }
                (*parent).next = new_ptr;

                // If this new node is the first node that follows the current one, it is the current's
                // first child.
                if (*parent).first_child.is_null() {
                    (*parent).first_child = new_ptr;
                }
            }

            // Save the new node ptr.
            self.cur_node = new_ptr;
            self.prev_new_node = new_ptr;
        }

        Ok(())
    }

    pub fn parsed_prop(
        &mut self,
        prop: &iters::ParsedProp<'dt>,
    ) -> Result<(), DevTreeError> {
        if !self.in_node_header {
            return Err(DevTreeError::ParseError);
        }

        unsafe {
            let new_ptr = self.allocate_aligned_ptr::<DTIProp>()?;
            (*self.cur_node).num_props += 1;
            *new_ptr = DTIProp::from(prop);
        }

        Ok(())
    }

    pub fn parsed_end_node(&mut self) -> Result<(), DevTreeError> {
        // There were more EndNode tokens than BeginNode ones.
        if self.cur_node.is_null() {
            return Err(DevTreeError::ParseError);
        }
        // Unsafe is Ok.
        // Lifetime : self.cur_node is a pointer into a buffer with the same lifetime as self
        // Alignment: parsed_node verifies alignment when creating self.cur_node
        // NonNull  : We check that self.cur_node is non-null above
        unsafe {
            // Change the current node back to the parent.
            self.cur_node = (*self.cur_node).parent;
        }

        // We are no longer in a node header.
        // We are either going to see a new node next or parse another end_node.
        self.in_node_header = false;

        Ok(())
    }
}

impl<'dt, 'i: 'dt> DevTreeIndex<'dt, 'i> {

    // Note: Our parsing method is unsafe - particularly due to its use of pointer arithmetic.
    //
    // We decide this is worth it for the following reasons:
    // - It requires no allocator.
    // - It has incredibly low overhead.
    //   - This parsing method only requires a single allocation. (The buffer given as buf)
    //   - This parsing method only requires a single iteration over the FDT.
    // - It is very easy to test in isolation; parsing is entirely enclosed to this module.
    unsafe fn init_builder(
        buf: &'i mut [u8],
        iter: &mut iters::DevTreeParseIter<'dt>,
    ) -> Result<DTIBuilder<'dt, 'i>, DevTreeError> {

        let mut builder = DTIBuilder {
            front_off: 0,
            buf,
            cur_node: null_mut(),
            prev_new_node: null_mut(),
            in_node_header: false,
        };

        for tok in iter {
            match tok {
                iters::ParsedTok::BeginNode(node) => {
                    builder.parsed_node(&node)?;
                    return Ok(builder);
                }
                iters::ParsedTok::Nop => continue,
                _ => return Err(DevTreeError::ParseError),
            }
        }

        Err(DevTreeError::ParseError)
    }

    pub fn get_layout(fdt: &'i DevTree<'dt>) -> Result<Layout, DevTreeError> {
        // Size may require alignment of DTINode.
        let mut size = 0usize;

        // We assert this because it makes size calculations easier.
        // We don't have to worry about re-aligning between props and nodes.
        // If they didn't have the same alignment, we would have to keep track
        // of the last node and re-align depending on the last seen type.
        //
        // E.g. If we saw one node, two props, and then two nodes:
        //
        // size = \
        // align_of::<DTINode> + size_of::<DTINode>
        // + align_of::<DTIProp> + size_of::<DTIProp>
        // + size_of::<DTIProp>
        // + size_of::<DTIProp>
        // + align_of::<DTINode> + size_of::<DTINode>
        // + size_of::<DTINode>
        const_assert_eq!(align_of::<DTINode>(), align_of::<DTIProp>());

        for item in iters::DevTreeIter::new(fdt) {
            match item {
                DevTreeItem::Node(_) => size += size_of::<DTINode>(),
                DevTreeItem::Prop(_) => size += size_of::<DTIProp>(),
            }
        }
        // TODO Check iter status

        // Unsafe okay.
        // - Size is not likely to be usize::MAX. (There's no way we find that many nodes.)
        // - Align is a result of align_of, so it will be a non-zero power of two
        unsafe {
            return Ok(Layout::from_size_align_unchecked(size, align_of::<DTINode>()));
        }
    }

    pub fn new(fdt: &'i DevTree<'dt>, buf: &'i mut [u8]) -> Result<Self, DevTreeError> {
        let mut iter = iters::DevTreeParseIter::new(fdt);

        let mut builder = unsafe {Self::init_builder(buf, &mut iter)}?;

        let this = Self {
            fdt,
            root: builder.cur_node,
        };

        // The buffer will be split into two parts, front and back:
        //
        // Front will be used as a temporary work section to  build the nodes as we parse them.
        // The back will be used to save completely parsed nodes.
        for tok in iter {
            match tok {
                iters::ParsedTok::BeginNode(node) => {
                    builder.parsed_node(&node)?;
                }
                iters::ParsedTok::Prop(prop) => {
                    builder.parsed_prop(&prop)?;
                }
                iters::ParsedTok::EndNode => {
                    builder.parsed_end_node()?;
                }
                iters::ParsedTok::Nop => continue,
            }
        }

        Ok(this)
    }

    /// Returns a [`DevTreeIndexDFSNodeIter`] over [`DevTreeIndexNode`] objects of the
    /// [`DevTreeIndex`]
    pub fn dfs_iter<'a>(&'a self) -> DevTreeIndexDFSNodeIter<'dt, 'i, 'a> {
        DevTreeIndexDFSNodeIter::new(self)
    }
}

#[derive(Clone)]
pub struct DevTreeIndexNode<'dt, 'i: 'dt, 'a: 'i> {
    pub index: &'a DevTreeIndex<'dt, 'i>,
    node: &'a DTINode<'dt, 'i>,
}

impl<'dt, 'i: 'dt, 'a: 'i> DevTreeIndexNode<'dt, 'i, 'a> {

    fn new(node: &'a DTINode<'dt, 'i>, index: &'a DevTreeIndex<'dt, 'i>) -> Self {
        Self { node, index }
    }

    pub fn name(&self) -> &'dt str {
        str::from_utf8(self.node.name).unwrap()
    }
}

/// An iterator over [`DevTreeIndexNode`] objects of the [`DevTreeIndex`]
#[derive(Clone)]
pub struct DevTreeIndexDFSNodeIter<'dt, 'i: 'dt, 'a: 'i> {
    pub index: &'a DevTreeIndex<'dt, 'i>,
    node: Option<&'a DTINode<'dt, 'i>>,
}

impl<'dt, 'i: 'dt, 'a: 'i> DevTreeIndexDFSNodeIter<'dt, 'i, 'a> {

    pub(crate) fn new(index: &'a DevTreeIndex<'dt, 'i>) -> Self {
        unsafe {
            let root_ref = index.root.as_ref().unsafe_unwrap();

            Self {
                index,
                node: Some(root_ref),
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

impl<'dt, 'i: 'dt, 'a: 'i> Iterator for DevTreeIndexDFSNodeIter<'dt, 'i, 'a> {

    type Item = DevTreeIndexNode<'dt, 'i, 'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.node {
            let cur = DevTreeIndexNode {
                index: self.index,
                node,
            };

            // Unsafe is OK.
            // This is unsafe because we cast node pointers into references.
            // These references live as long as the DevTreeIndex and therefore the lifetimes match.
            unsafe {
                self.node = if let Some(next) = node.first_child.as_ref() {
                    Some(next)
                } else if let Some(next) = node.next.as_ref() {
                    Some(next)
                } else {
                    None
                }
            }
            return Some(cur);
        }
        None
    }
}

// TODO Fuck load of utility methods.

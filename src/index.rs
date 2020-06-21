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
use crate::iters::AssociatedOffset;
use crate::unsafe_unwrap::UnsafeUnwrap;
use crate::*;

use core::marker::PhantomData;

// TODO Add a wrapper around these that is easier to use (that includes a reference to the fdt).
pub struct DTIProp<'dt> {
    propbuf: &'dt [u8],
    nameoff: AssociatedOffset<'dt>,
}

impl<'dt> From<iters::ParsedProp<'dt>> for DTIProp<'dt> {
    fn from(prop: iters::ParsedProp<'dt>) -> Self {
        Self {
            propbuf: prop.prop_buf,
            nameoff: prop.name_offset,
        }
    }
}

// TODO Add a wrapper around these that is easier to use (that includes a reference to the fdt).
struct DTINode<'dt, 'i: 'dt> {
    parent: Option<*const DTINode<'dt, 'i>>,
    children: Vec<DTINode<'dt, 'i>>,
    props: Vec<DTIProp<'dt>>,
    name: &'dt [u8],
    _index: PhantomData<&'i [u8]>,
}

impl<'dt, 'i: 'dt> DTINode<'dt, 'i> {
    fn new(
        parent: Option<*const DTINode<'dt, 'i>>,
        node: iters::ParsedBeginNode<'dt>,
    ) -> Self {
        Self {
            parent,
            children: Vec::new(),
            props: Vec::new(),
            name: node.name,
            _index: PhantomData,
        }
    }
}

pub struct DevTreeIndex<'dt, 'i: 'dt> {
    fdt: &'i DevTree<'dt>,
    root: Box<DTINode<'dt, 'i>>,
}

impl<'dt, 'i: 'dt> DevTreeIndex<'dt, 'i> {
    fn get_root_node(
        iter: &mut iters::DevTreeParseIter<'dt>,
    ) -> Result<Box<DTINode<'dt, 'i>>, DevTreeError> {
        // Prime the initial current_node
        for tok in iter {
            match tok {
                iters::ParsedTok::BeginNode(node) => {
                    return Ok(Box::new(DTINode::new(None, node)))
                }
                iters::ParsedTok::Nop => (),
                _ => return Err(DevTreeError::ParseError),
            }
        }
        Err(DevTreeError::ParseError)
    }

    pub fn new(fdt: &'i DevTree<'dt>) -> Result<Self, DevTreeError> {
        let mut iter = iters::DevTreeParseIter::new(fdt);

        let mut root = Self::get_root_node(&mut iter)?;
        let mut cur_node = root.as_mut();

        // Devtree Props may only occur before child nodes. We'll call this the "node_header".
        let mut in_node_header = true;

        for tok in iter {
            match tok {
                iters::ParsedTok::BeginNode(node) => {
                    // Allocate node from parsed node.
                    cur_node
                        .children
                        .push(DTINode::new(Some(cur_node), node));
                    // (Unwrap safe, we just pushed a node.)
                    unsafe {
                        cur_node = cur_node.children.last_mut().unsafe_unwrap();
                    }
                }
                iters::ParsedTok::Prop(prop) => {
                    if !in_node_header {
                        return Err(DevTreeError::ParseError);
                    }
                    cur_node.props.push(DTIProp::from(prop));
                }
                iters::ParsedTok::EndNode => {
                    // Cast the current node's *const parent pointer into a mutable reference.
                    //
                    // This is safe because this will be the only mutable reference to the parent
                    // while this function is active.
                    //
                    // We believe this to not violate the aliasing rules as they are currently
                    // defined. This soon to be mutable reference is the only way we access parent
                    // while the mutable reference exists.
                    //
                    // Quote: https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html
                    //
                    // The precise Rust aliasing rules are somewhat in flux, but the main points
                    // are not contentious:
                    //
                    //  -  If you create a safe reference with lifetime 'a (either a &T or &mut
                    //     T reference) that is accessible by safe code (for example, because you
                    //     returned it), then you must not access the data in any way that
                    //     contradicts that reference for the remainder of 'a. For example, this
                    //     means that if you take the *mut T from an UnsafeCell<T> and cast it to
                    //     an &T, then the data
                    //     in T must remain immutable (modulo any UnsafeCell data found within T,
                    //     of course) until that reference's lifetime expires. Similarly, if you
                    //     create a &mut T reference that is released to safe code, then you must
                    //     not access the data within the UnsafeCell until that reference expires.
                    unsafe {
                        let n_ref = cur_node.parent.ok_or(DevTreeError::ParseError)?;
                        cur_node = &mut *(n_ref as *mut DTINode<'dt, 'i>);
                    }
                    in_node_header = false;
                }
                iters::ParsedTok::Nop => continue,
            }
        }

        Ok(Self { root, fdt })
    }
}

//pub struct DevTreeIndexNode<'dt, 'i: 'dt> {
//    pub index: &'i DevTreeIndex<'dt, 'i>,
//    node: &'i DTINode<'dt, 'i>,
//}
//
//impl<'dt, 'i: 'dt> DevTreeIndexNode<'dt, 'i> {
//    fn new(node: &'i DTINode<'dt, 'i>, index: DevTreeIndex<'dt, 'i>) -> Self {
//        Self {
//            node,
//            index
//        }
//    }
//}
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
}

// TODO Note iterator
/// An interator over [`DevTreeNode`] objects in the [`DevTree`]
#[derive(Clone)]
pub struct DevTreeIndexNodeIter<'dt, 'i: 'dt, 'a: 'i> {
    index_node: DevTreeIndexNode<'dt, 'i, 'a>
}

impl<'dt, 'i: 'dt, 'a: 'i> DevTreeIndexNodeIter<'dt ,'i, 'a> {
    pub(crate) fn new(index: &'a DevTreeIndex<'dt, 'i>) -> Self {
        Self {
            index_node: DevTreeIndexNode::new(index.root.as_ref(), index)
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
        todo!()
    }
}

// TODO Fuck load of utility methods.

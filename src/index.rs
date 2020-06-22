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

unsafe fn ptr_in<T>(buf: &[u8], ptr: *const T) -> bool {
    buf.as_ptr().add(buf.len()) < (ptr as *const u8)
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
struct DTINode<'dt, 'i: 'dt> {
    parent: Option<*const DTINode<'dt, 'i>>,
    children: &'i [DTINode<'dt, 'i>],
    props: &'i [DTIProp<'dt>],
    name: &'dt [u8],
}

struct DTIWorkingNode<'dt, 'i: 'dt> {
    parent: *mut DTIWorkingNode<'dt, 'i>,
    children: usize,
    props: usize,
    name: &'dt [u8],
}

impl<'dt, 'i: 'dt> DTIWorkingNode<'dt, 'i> {
}


pub struct DevTreeIndex<'dt, 'i: 'dt> {
    fdt: &'i DevTree<'dt>,
    buf: &'i mut [u8],
    // TODO root
}

struct DevTreeIndexBuildState<'dt, 'i: 'dt> {
    buf: &'i mut [u8],
    cur_node: *mut DTIWorkingNode<'dt, 'i>,
    front_off: usize,
    back_off: usize,
}

impl<'dt, 'i:'dt> DevTreeIndexBuildState<'dt, 'i> {
    pub unsafe fn append_node(&mut self, node: &iters::ParsedBeginNode<'dt>) {
        // Align pointer
        let ptr = (self.front_off as *mut u8).align_offset(align_of::<DTIWorkingNode>()) as *mut DTIWorkingNode;
        assert!(ptr_in(self.buf, ptr));

        if self.cur_node != null_mut() {
            (*self.cur_node).children += 1;
        }

        // Write the data
        *ptr = DTIWorkingNode {
            parent: self.cur_node,
            children: 0,
            props: 0,
            name: node.name,
        };

        // Save the new node ptr
        self.cur_node = ptr;
        // Increment offset
        self.front_off = ptr.add(1) as usize;

        assert!(self.front_off <= self.back_off);
    }

    pub unsafe fn append_prop(&mut self, prop: &iters::ParsedProp<'dt>) {
        // Align pointer
        let ptr = (self.front_off as *mut u8).align_offset(align_of::<DTIProp>()) as *mut DTIProp;
        assert!(ptr_in(self.buf, ptr));

        // Write the data
        *ptr = DTIProp::from(prop);

        // Update the Node's prop list
        (*self.cur_node).props += 1;

        // Increment offset
        self.front_off = ptr.add(1) as usize;

        assert!(self.front_off <= self.back_off);
    }

    pub unsafe fn freeze_current_node(&mut self) {
        // Align pointer
        let mut ptr = (self.back_off as *mut u8).align_offset(align_of::<DTINode>()) as *mut DTINode;
        // Move back pointer
        ptr = ptr.sub(1);
        assert!(ptr_in(self.buf, ptr));

        // Write in the current node
        self.write_frozen_node(ptr);

        self.back_off = ptr as usize;
    }

    fn write_frozen_node(&self, ptr: *mut DTINode) {
        // TODO Let's move the node into perminent section in the back.
        // - TODO: Let's move the node shell
        // - TODO: Let's move the node's props
        // - TODO Fixup all parent links in our children.
    }
}

impl<'dt, 'i: 'dt> DevTreeIndex<'dt, 'i> {
    unsafe fn init_state(&'i mut self, iter: &mut iters::DevTreeParseIter<'dt>) -> Result<DevTreeIndexBuildState<'dt, 'i>, DevTreeError> {
        let mut state = DevTreeIndexBuildState {
            front_off: 0,
            back_off: self.buf.len() -1,
            buf: self.buf,
            cur_node: null_mut(),
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

    // Note: This parsing method is particularly unsafe. We decide this is worth it because it
    // enables running this method without any allocator and uses minimal overhead.
    pub unsafe fn new(fdt: &'i DevTree<'dt>, buf: &'i mut [u8]) -> Result<Self, DevTreeError> {
        let mut this = Self {
            fdt, buf,
        };
        let mut iter = iters::DevTreeParseIter::new(this.fdt);

        let mut state = this.init_state(&mut iter)?;

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

                    state.freeze_current_node();

                }
                iters::ParsedTok::Nop => continue,
            }
        }

        todo!()
    }
}

////pub struct DevTreeIndexNode<'dt, 'i: 'dt> {
////    pub index: &'i DevTreeIndex<'dt, 'i>,
////    node: &'i DTINode<'dt, 'i>,
////}
////
////impl<'dt, 'i: 'dt> DevTreeIndexNode<'dt, 'i> {
////    fn new(node: &'i DTINode<'dt, 'i>, index: DevTreeIndex<'dt, 'i>) -> Self {
////        Self {
////            node,
////            index
////        }
////    }
////}
//#[derive(Clone)]
//pub struct DevTreeIndexNode<'dt, 'i: 'dt, 'a: 'i> {
//    pub index: &'a DevTreeIndex<'dt, 'i>,
//    node: &'a DTINode<'dt, 'i>,
//}
//
//impl<'dt, 'i: 'dt, 'a: 'i> DevTreeIndexNode<'dt, 'i, 'a> {
//    fn new(node: &'a DTINode<'dt, 'i>, index: &'a DevTreeIndex<'dt, 'i>) -> Self {
//        Self {
//            node,
//            index
//        }
//    }
//}
//
//// TODO Note iterator
///// An interator over [`DevTreeNode`] objects in the [`DevTree`]
//#[derive(Clone)]
//pub struct DevTreeIndexNodeIter<'dt, 'i: 'dt, 'a: 'i> {
//    index_node: DevTreeIndexNode<'dt, 'i, 'a>
//}
//
//impl<'dt, 'i: 'dt, 'a: 'i> DevTreeIndexNodeIter<'dt ,'i, 'a> {
//    pub(crate) fn new(index: &'a DevTreeIndex<'dt, 'i>) -> Self {
//        Self {
//            index_node: DevTreeIndexNode::new(index.root.as_ref(), index)
//        }
//    }
//
//    // See the documentation of [`DevTree::find_node`]
//    //#[inline]
//    //pub fn find<F>(&mut self, predicate: F) -> Option<(DevTreeNode<'a>, Self)>
//    //where
//    //    F: Fn(&DevTreeNode) -> Result<bool, DevTreeError>,
//    //{
//    //}
//}
//
//impl<'dt, 'i: 'dt, 'a: 'i> Iterator for DevTreeIndexNodeIter<'dt ,'i, 'a> {
//    type Item = DevTreeIndexNode<'dt ,'i, 'a>;
//    fn next(&mut self) -> Option<Self::Item> {
//        todo!()
//    }
//}

// TODO Fuck load of utility methods.

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

// TODO Rather than using a "real" allocator, we could do something in between the real allocator
// and a fake one.
//
// The allocator would start with a `&mut [u8]` hunk of memory, allocate all data as completely
// packed data, but provide aligned values in return (some space will be lost).
use crate::iters::AssociatedOffset;
use crate::unsafe_unwrap::UnsafeUnwrap;
use crate::*;

use core::marker::PhantomData;

// TODO Add a wrapper around these that is easier to use (that includes a reference to the fdt).
pub struct DevTreeIndexProp<'dt> {
    propbuf: &'dt [u8],
    nameoff: AssociatedOffset<'dt>,
}

impl<'dt> From<iters::ParsedProp<'dt>> for DevTreeIndexProp<'dt> {
    fn from(prop: iters::ParsedProp<'dt>) -> Self {
        Self {
            propbuf: prop.prop_buf,
            nameoff: prop.name_offset,
        }
    }
}

// TODO Add a wrapper around these that is easier to use (that includes a reference to the fdt).
/* From: https://doc.rust-lang.org/std/cell/struct.UnsafeCell.html
 *
 * The precise Rust aliasing rules are somewhat in flux, but the main points are not contentious:
 *
 *  -  If you create a safe reference with lifetime 'a (either a &T or &mut T reference) that is
 *     accessible by safe code (for example, because you returned it), then you must not access the
 *     data in any way that contradicts that reference for the remainder of 'a. For example, this
 *     means that if you take the *mut T from an UnsafeCell<T> and cast it to an &T, then the data
 *     in T must remain immutable (modulo any UnsafeCell data found within T, of course) until that
 *     reference's lifetime expires. Similarly, if you create a &mut T reference that is released to
 *     safe code, then you must not access the data within the UnsafeCell until that reference
 *     expires.
 *
 *  -  At all times, you must avoid data races. If multiple threads have access to the same
 *     UnsafeCell, then any writes must have a proper happens-before relation to all other accesses
 *     (or use atomics).
 *
 * We hold a *const to our parent, but when creating the index, we bump this to a mutable
 * one. The rust spec seems to allow this.
 */
pub struct DevTreeIndexNode<'dt, 'i: 'dt> {
    parent: Option<*const DevTreeIndexNode<'dt, 'i>>,
    children: Vec<DevTreeIndexNode<'dt, 'i>>,
    props: Vec<DevTreeIndexProp<'dt>>,
    name: &'dt [u8],
    _index: PhantomData<&'i [u8]>,
}

/// TODO
pub struct DevTreeIndex<'dt, 'i: 'dt> {
    fdt: &'i DevTree<'dt>,
    root: Box<DevTreeIndexNode<'dt, 'i>>,
}

impl<'dt, 'i: 'dt> DevTreeIndex<'dt, 'i> {
    fn get_root_node(
        iter: &mut iters::DevTreeParseIter<'dt>,
    ) -> Result<Box<DevTreeIndexNode<'dt, 'i>>, DevTreeError> {
        // Prime the initial current_node
        for tok in iter {
            match tok {
                iters::ParsedTok::BeginNode(node) => {
                    return Ok(Box::new(DevTreeIndexNode {
                        parent: None,
                        children: Vec::new(),
                        props: Vec::new(),
                        name: node.name,
                        _index: PhantomData,
                    }))
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
                    cur_node.children.push(DevTreeIndexNode {
                        parent: Some(cur_node),
                        children: Vec::new(),
                        props: Vec::new(),
                        name: node.name,
                        _index: PhantomData,
                    });
                    // (Unwrap safe, we just pushed a node.)
                    unsafe {
                        cur_node = cur_node.children.last_mut().unsafe_unwrap();
                    }
                }
                iters::ParsedTok::Prop(prop) => {
                    if !in_node_header {
                        return Err(DevTreeError::ParseError);
                    }
                    cur_node.props.push(DevTreeIndexProp::from(prop));
                }
                iters::ParsedTok::EndNode => {
                    let cur_node = cur_node.parent.ok_or(DevTreeError::ParseError)?;
                    in_node_header = false;
                }
                iters::ParsedTok::Nop => continue,
            }
        }

        Ok(Self { root, fdt })
    }
}

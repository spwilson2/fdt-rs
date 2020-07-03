//! This module provides a collection of iterative parsers of the buf provided to initialize
//! a [`DevTree`].
use core::mem::size_of;
use core::num::NonZeroUsize;
use core::str::from_utf8;

use crate::base::parse::{next_devtree_token, ParsedTok};
use crate::base::{DevTree, DevTreeItem, DevTreeNode, DevTreeProp};
use crate::error::DevTreeError;
use crate::prelude::*;
use crate::spec::fdt_reserve_entry;

/// An iterator over [`fdt_reserve_entry`] objects within the FDT.
#[derive(Clone)]
pub struct DevTreeReserveEntryIter<'a> {
    offset: usize,
    fdt: &'a DevTree<'a>,
}

impl<'a> DevTreeReserveEntryIter<'a> {
    pub(crate) fn new(fdt: &'a DevTree) -> Self {
        Self {
            offset: fdt.off_mem_rsvmap(),
            fdt,
        }
    }

    /// Return the current offset as a fdt_reserve_entry reference.
    ///
    /// # Safety
    ///
    /// The caller must verify that the current offset of this iterator is 32-bit aligned.
    /// (Each field is 32-bit aligned and they may be read individually.)
    unsafe fn read(&self) -> Result<&'a fdt_reserve_entry, DevTreeError> {
        Ok(&*self.fdt.ptr_at(self.offset)?)
    }
}

impl<'a> Iterator for DevTreeReserveEntryIter<'a> {
    type Item = &'a fdt_reserve_entry;
    fn next(&mut self) -> Option<Self::Item> {
        if self.offset > self.fdt.totalsize() {
            None
        } else {
            // We guaruntee the read will be aligned to 32 bytes because:
            // - We construct with guarunteed 32-bit aligned offset
            // - We always increment by an aligned amount
            let ret = unsafe { self.read().unwrap() };

            if ret.address == 0.into() && ret.size == 0.into() {
                return None;
            }
            self.offset += size_of::<fdt_reserve_entry>();
            Some(ret)
        }
    }
}

/// An iterator over all [`DevTreeItem`] objects.
#[derive(Clone)]
pub struct DevTreeIter<'a> {
    /// Offset of the last opened Device Tree Node.
    /// This is used to set properties' parent DevTreeNode.
    ///
    /// As defined by the spec, DevTreeProps must preceed Node definitions.
    /// Therefore, once a node has been closed this offset is reset to None to indicate no
    /// properties should follow.
    current_prop_parent_off: Option<NonZeroUsize>,

    /// Current offset into the flattened dt_struct section of the device tree.
    offset: usize,
    pub(crate) fdt: &'a DevTree<'a>,
    //parse_error: Option<>
}

def_common_iter_funcs!($ DevTreeNode<'a>, DevTreeProp<'a>, DevTreeNodeIter, DevTreePropIter, DevTreeItem);

impl<'a> DevTreeIter<'a> {
    pub fn new(fdt: &'a DevTree) -> Self {
        Self {
            offset: fdt.off_dt_struct(),
            current_prop_parent_off: None,
            fdt,
        }
    }

    fn current_node_itr(&self) -> Option<DevTreeIter<'a>> {
        match self.current_prop_parent_off {
            Some(offset) => Some(DevTreeIter {
                fdt: self.fdt,
                current_prop_parent_off: self.current_prop_parent_off,
                offset: offset.get(),
            }),
            None => None,
        }
    }

    fn_next_node!(
        /// Returns the next [`DevTreeNode`] found in the Device Tree
    );

    fn_next_prop!(
        /// Returns the next [`DevTreeProp`] found in the [`DevTree`]. This property may be on
        /// a different [`DevTreeNode`] than the previous property.
        ///
        /// (See [`next_node_prop`] if a property should be returned only if it exists on this
        /// node.)
        ///
        /// [`next_node_prop`]: #DevTreeIter::next_node_prop
    );

    fn_next_node_prop!(
        /// Returns the next [`DevTreeProp`] of the current [`DevTreeNode`] or `None` if
        /// the node does not have any more properties.
    );

    fn_find_next_compatible_node!(
        /// Returns the next [`DevTreeNode`] object with the provided compatible device tree property
        /// or [`Option::None`] if none exists.
    );

    fn next_devtree_item(&mut self) -> Option<DevTreeItem<'a>> {
        loop {
            let old_offset = self.offset;
            // Safe because we only pass offsets which are returned by next_devtree_token.
            let res = unsafe { next_devtree_token(self.fdt.buf(), &mut self.offset) };
            match res {
                Ok(Some(ParsedTok::BeginNode(node))) => {
                    self.current_prop_parent_off =
                        unsafe { Some(NonZeroUsize::new_unchecked(old_offset)) };
                    return Some(DevTreeItem::Node(DevTreeNode {
                        parse_iter: self.clone(),
                        name: from_utf8(node.name).map_err(|e| e.into()),
                    }));
                }
                Ok(Some(ParsedTok::Prop(prop))) => {
                    // Prop must come after a node.
                    let prev_node = match self.current_node_itr() {
                        Some(n) => n,
                        None => return None, // Devtree error - end iteration
                    };
                    return Some(DevTreeItem::Prop(DevTreeProp::new(
                        prev_node,
                        prop.prop_buf,
                        prop.name_offset,
                    )));
                }
                Ok(Some(ParsedTok::EndNode)) => {
                    // The current node has ended.
                    // No properties may follow until the next node starts.
                    self.current_prop_parent_off = None;
                }
                Ok(Some(_)) => continue,
                _ => return None,
            }
        }
    }
}

impl<'a> Iterator for DevTreeIter<'a> {
    type Item = DevTreeItem<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.next_devtree_item()
    }
}

/// An iterator over [`DevTreeNode`] objects in the [`DevTree`]
#[derive(Clone)]
pub struct DevTreeNodeIter<'a>(DevTreeIter<'a>);

impl<'a> DevTreeNodeIter<'a> {
    pub(crate) fn new(fdt: &'a DevTree) -> Self {
        Self(DevTreeIter::new(fdt))
    }
}

impl<'a> Iterator for DevTreeNodeIter<'a> {
    type Item = DevTreeNode<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_node()
    }
}

impl<'a> From<DevTreeIter<'a>> for DevTreeNodeIter<'a> {
    fn from(iter: DevTreeIter<'a>) -> Self {
        Self(iter)
    }
}

/// An iterator over [`DevTreeProp`] objects in the [`DevTree`]
#[derive(Clone)]
pub struct DevTreePropIter<'a>(DevTreeIter<'a>);

impl<'a> Iterator for DevTreePropIter<'a> {
    type Item = DevTreeProp<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_prop()
    }
}

impl<'a> From<DevTreeIter<'a>> for DevTreePropIter<'a> {
    fn from(iter: DevTreeIter<'a>) -> Self {
        Self(iter)
    }
}

/// An iterator over [`DevTreeProp`] objects on a single node within the [`DevTree`]
#[derive(Clone)]
pub struct DevTreeNodePropIter<'a>(DevTreeIter<'a>);

impl<'a> DevTreeNodePropIter<'a> {
    pub(crate) fn new(node: &'a DevTreeNode) -> Self {
        Self(node.parse_iter.clone())
    }
}

impl<'a> Iterator for DevTreeNodePropIter<'a> {
    type Item = DevTreeProp<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_node_prop()
    }
}

impl<'a> From<DevTreeIter<'a>> for DevTreeNodePropIter<'a> {
    fn from(iter: DevTreeIter<'a>) -> Self {
        Self(iter)
    }
}

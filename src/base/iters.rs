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
pub struct DevTreeReserveEntryIter<'a, 'dt:'a> {
    offset: usize,
    fdt: &'a DevTree<'dt>,
}

impl<'a, 'dt:'a> DevTreeReserveEntryIter<'a, 'dt> {
    pub(crate) fn new(fdt: &'a DevTree<'dt>) -> Self {
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
    unsafe fn read(&'a self) -> Result<&'dt fdt_reserve_entry, DevTreeError> {
        Ok(&*self.fdt.ptr_at(self.offset)?)
    }
}

impl<'a, 'dt: 'a> Iterator for DevTreeReserveEntryIter<'a, 'dt> {
    type Item = &'dt fdt_reserve_entry;
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
pub struct DevTreeIter<'a, 'dt:'a> {
    /// Offset of the last opened Device Tree Node.
    /// This is used to set properties' parent DevTreeNode.
    ///
    /// As defined by the spec, DevTreeProps must preceed Node definitions.
    /// Therefore, once a node has been closed this offset is reset to None to indicate no
    /// properties should follow.
    current_prop_parent_off: Option<NonZeroUsize>,

    /// Current offset into the flattened dt_struct section of the device tree.
    offset: usize,
    pub(crate) fdt: &'a DevTree<'dt>,
}

//def_common_iter_funcs!($ DevTreeNode<'a, 'dt>, DevTreeProp<'a, 'dt>, DevTreeNodeIter, DevTreePropIter, DevTreeItem);
use crate::base::item::UnwrappableDevTreeItem;

impl<'a, 'dt: 'a> ItemIterator<'a, 'dt, DevTreeItem<'a, 'dt>> for DevTreeIter<'a, 'dt> {
    type TreeNodeIter = DevTreeNodeIter<'a, 'dt>;
    type TreePropIter = DevTreePropIter<'a, 'dt>;
}

impl<'a, 'dt:'a> DevTreeIter<'a, 'dt> {
    pub fn new(fdt: &'a DevTree<'dt>) -> Self {
        Self {
            offset: fdt.off_dt_struct(),
            current_prop_parent_off: None,
            fdt,
        }
    }

    fn current_node_itr(&self) -> Option<DevTreeIter<'a, 'dt>> {
        match self.current_prop_parent_off {
            Some(offset) => Some(DevTreeIter {
                fdt: self.fdt,
                current_prop_parent_off: self.current_prop_parent_off,
                offset: offset.get(),
            }),
            None => None,
        }
    }

    pub fn next_prop(&mut self) -> Option<DevTreeProp<'a, 'dt>> {
        loop {
            match self.next() {
                Some(item) => {
                    if let Some(prop) = item.prop() {
                        return Some(prop)
                    }
                    // Continue if a new node.
                    continue
                },
                _ => return None,
            }
        }
    }

    fn next_devtree_item(&mut self) -> Option<DevTreeItem<'a, 'dt>> {
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

impl<'a, 'dt:'a> Iterator for DevTreeIter<'a, 'dt> {
    type Item = DevTreeItem<'a, 'dt>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.next_devtree_item()
    }
}

/// An iterator over [`DevTreeNode`] objects in the [`DevTree`]
#[derive(Clone)]
pub struct DevTreeNodeIter<'a, 'dt:'a>(DevTreeIter<'a, 'dt>);

impl<'a, 'dt:'a> Iterator for DevTreeNodeIter<'a, 'dt> {
    type Item = DevTreeNode<'a, 'dt>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_node()
    }
}

impl<'a, 'dt:'a> From<DevTreeIter<'a, 'dt>> for DevTreeNodeIter<'a, 'dt> {
    fn from(iter: DevTreeIter<'a, 'dt>) -> Self {
        Self(iter)
    }
}

/// An iterator over [`DevTreeProp`] objects in the [`DevTree`]
#[derive(Clone)]
pub struct DevTreePropIter<'a, 'dt:'a>(DevTreeIter<'a, 'dt>);

impl<'a, 'dt:'a> Iterator for DevTreePropIter<'a, 'dt> {
    type Item = DevTreeProp<'a, 'dt>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_prop()
    }
}

impl<'a, 'dt:'a> From<DevTreeIter<'a, 'dt>> for DevTreePropIter<'a, 'dt> {
    fn from(iter: DevTreeIter<'a, 'dt>) -> Self {
        Self(iter)
    }
}

impl<'a, 'dt:'a> From<DevTreeNodeIter<'a, 'dt>> for DevTreePropIter<'a, 'dt> {
    fn from(iter: DevTreeNodeIter<'a, 'dt>) -> Self {
        Self(iter.0)
    }
}

/// An iterator over [`DevTreeProp`] objects on a single node within the [`DevTree`]
#[derive(Clone)]
pub struct DevTreeNodePropIter<'a, 'dt:'a>(DevTreeIter<'a, 'dt>);

impl<'a, 'dt:'a> DevTreeNodePropIter<'a, 'dt> {
    pub(crate) fn new(node: &DevTreeNode<'a, 'dt>) -> Self {
        Self(node.parse_iter.clone())
    }
}

impl<'a, 'dt:'a> Iterator for DevTreeNodePropIter<'a, 'dt> {
    type Item = DevTreeProp<'a, 'dt>;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_node_prop()
    }
}

impl<'a, 'dt:'a> From<DevTreeIter<'a, 'dt>> for DevTreeNodePropIter<'a, 'dt> {
    fn from(iter: DevTreeIter<'a, 'dt>) -> Self {
        Self(iter)
    }
}

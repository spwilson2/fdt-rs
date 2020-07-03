//! This module provides a collection of iterative parsers of the buf provided to initialze
//! a [`DevTree`].
use core::mem::size_of;
use core::num::NonZeroUsize;
use core::str::from_utf8;

use crate::base::parse::{next_devtree_token, ParsedTok};
use crate::base::{DevTree, DevTreeItem, DevTreeNode, DevTreeProp};
use crate::error::DevTreeError;
use crate::prelude::*;
use crate::spec::fdt_reserve_entry;

pub trait FindNext: Iterator + core::clone::Clone {
    #[inline]
    fn find_next<F>(&mut self, predicate: F) -> Option<(Self::Item, Self)>
    where
        F: Fn(&Self::Item) -> Result<bool, DevTreeError>,
        <Self as Iterator>::Item: core::marker::Sized,
        Self: core::marker::Sized,
    {
        while let Some(i) = self.next() {
            if let Ok(true) = predicate(&i) {
                return Some((i, self.clone()));
            }
        }
        None
    }
}

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

impl FindNext for DevTreeIter<'_> {}
impl<'a> DevTreeIter<'a> {
    pub(crate) fn new(fdt: &'a DevTree) -> Self {
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

    /// Returns the next [`DevTreeNode`] found in the Device Tree
    #[inline]
    pub fn next_node(&mut self) -> Option<DevTreeNode<'a>> {
        loop {
            match self.next() {
                Some(DevTreeItem::Node(n)) => return Some(n),
                Some(_) => {
                    continue;
                }
                _ => return None,
            }
        }
    }

    /// Returns the next [`DevTreeProp`] found in the Device Tree (regardless if it occurs on
    /// a different [`DevTreeNode`]
    #[inline]
    pub fn next_prop(&mut self) -> Option<DevTreeProp<'a>> {
        loop {
            match self.next() {
                Some(DevTreeItem::Prop(p)) => return Some(p),
                // Return if a new node or an EOF.
                Some(DevTreeItem::Node(_)) => continue,
                _ => return None,
            }
        }
    }

    /// Returns the next [`DevTreeProp`] on the current node within in the Device Tree
    #[inline]
    pub fn next_node_prop(&mut self) -> Option<DevTreeProp<'a>> {
        match self.next() {
            Some(DevTreeItem::Prop(p)) => Some(p),
            // Return if a new node or an EOF.
            _ => None,
        }
    }

    /// Returns the next [`DevTreeNode`] object with the provided compatible device tree property
    /// or `None` if none exists.
    #[inline]
    pub fn find_next_compatible_node(&self, string: &str) -> Option<DevTreeNode<'a>> {
        // Create a clone and turn it into a node iterator
        let mut iter = DevTreeNodeIter::from(self.clone());
        // If there is another node
        if iter.next().is_some() {
            // Iterate through its properties looking for the compatible string.
            let mut iter = DevTreePropIter::from(iter.0);
            if let Some((compatible_prop, _)) = iter.find_next(|prop| unsafe {
                Ok((prop.name()? == "compatible") && (prop.get_str()? == string))
            }) {
                return Some(compatible_prop.node());
            }
        }
        None
    }

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

impl FindNext for DevTreeNodeIter<'_> {}
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

impl FindNext for DevTreePropIter<'_> {}
impl<'a> DevTreePropIter<'a> {
    pub(crate) fn new(fdt: &'a DevTree) -> Self {
        Self(DevTreeIter::new(fdt))
    }
}

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

impl FindNext for DevTreeNodePropIter<'_> {}
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

//! This module provides a macro which defines common methods for both the [`DevTreeIter`] and
//! the [`DevTreeIndexIter`]
//!
//! Iteration between both the raw [`DevTree`] and the indexed [`DevTreeIndex`] utilizes many of
//! the same algorithms. These algorithms use common implementations which only differ by types.
#[cfg(doc)]
use crate::base::iters::*;
#[cfg(doc)]
use crate::base::*;
#[cfg(doc)]
use crate::index::iters::*;
#[cfg(doc)]
use crate::index::*;

use crate::prelude::*;

use super::item::UnwrappableDevTreeItem;

pub trait ItemIterator<'r, 'dt: 'r, I>: Clone + Iterator<Item=I> where I: UnwrappableDevTreeItem<'dt> {
    type TreeNodeIter: From<Self> + Iterator<Item=I::TreeNode>;
    type TreePropIter: From<Self::TreeNodeIter> + Iterator<Item=I::TreeProp>;

    fn next_prop(&mut self) -> Option<I::TreeProp> {
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

    fn next_node(&mut self) -> Option<I::TreeNode> {
        loop {
            match self.next() {
                Some(item) => {
                    if let Some(node) = item.node() {
                        return Some(node)
                    }
                    // Continue if a new prop.
                    continue
                },
                _ => return None,
            }
        }
    }

    fn next_node_prop(&mut self) -> Option<I::TreeProp> {
        match self.next() {
            // Return if a new node or an EOF.
            Some(item) => item.prop(),
            _ => None,
        }
    }

    fn find_next_compatible_node(&self, string: &str) -> Option<<I::TreeProp as DevTreePropStateBase<'dt>>::NodeType> {
        // Create a clone and turn it into a node iterator
        let mut node_iter = Self::TreeNodeIter::from(self.clone());

        // If there is another node, advance our iterator to that node.
        node_iter.next().and_then(|_| {

            // Iterate through all remaining properties in the tree looking for the compatible
            // string.
            let mut iter = Self::TreePropIter::from(node_iter);
            iter.find_map(|prop| unsafe {

                // Verify that the compatible prop matches
                if prop.name().ok()? == "compatible" && prop.get_str().ok()? == string {
                    return Some(prop);
                }
                None

            }).and_then(|compatible_prop| {
                // If we found a compatible property match, return the node.
                return Some(compatible_prop.node());
            })
        })
    }
}

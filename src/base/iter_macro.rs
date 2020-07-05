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

use super::item::UnwrappableDevTreeItem;

pub trait ItemIterator {
    type TreeProp;
    type TreeNode;
    type TreeItem: Iterator + UnwrappableDevTreeItem;

    fn next_prop(&mut iter: Self::TreeItem) -> Option<Self::TreeProp> {
        loop {
            match iter.next() {
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
}

//macro_rules! def_common_iter_funcs {
//    ( $esc:tt
//      $TreeNode:ty,
//      $TreeProp:ty,
//      $TreeNodeIter:ident,
//      $TreePropIter:ident,
//      $TreeItem:ident) => {
//
//        macro_rules! fn_next_node {
//            ($esc(#[$attr:meta])*) => {
//
//                $esc(#[$attr])*
//                #[inline]
//                pub fn next_node(&mut self) -> Option<$TreeNode> {
//                    loop {
//                        match self.next() {
//                            Some($TreeItem::Node(n)) => return Some(n),
//                            Some(_) => {
//                                continue;
//                            }
//                            _ => return None,
//                        }
//                    }
//                }
//            }
//        }
//
//        macro_rules! fn_next_prop {
//            ($esc(#[$attr:meta])*) => {
//                $esc(#[$attr])*
//                    #[inline]
//                    pub fn next_prop(&mut self) -> Option<$TreeProp> {
//                        loop {
//                            match self.next() {
//                                Some($TreeItem::Prop(p)) => return Some(p),
//                                // Return if a new node or an EOF.
//                                Some($TreeItem::Node(_)) => continue,
//                                _ => return None,
//                            }
//                        }
//                    }
//            }
//        }
//
//        macro_rules! fn_next_node_prop {
//            ($esc(#[$attr:meta])*) => {
//
//                $esc(#[$attr])*
//                    #[inline]
//                    pub fn next_node_prop(&mut self) -> Option<$TreeProp> {
//                        match self.next() {
//                            Some($TreeItem::Prop(p)) => Some(p),
//                            // Return if a new node or an EOF.
//                            _ => None,
//                        }
//                    }
//            }
//        }
//
//        macro_rules! fn_find_next_compatible_node {
//            ($esc(#[$attr:meta])*) => {
//                $esc(#[$attr])*
//                    #[inline]
//                    pub fn find_next_compatible_node(&self, string: &str) -> Option<$TreeNode> {
//                        // Create a clone and turn it into a node iterator
//                        let mut node_iter = $TreeNodeIter::from(self.clone());
//
//                        // If there is another node, advance our iterator to that node.
//                        node_iter.next().and_then(|_| {
//
//                            // Iterate through all remaining properties in the tree looking for the compatible
//                            // string.
//                            let mut iter = $TreePropIter::from(node_iter.0);
//                            iter.find_map(|prop| unsafe {
//
//                                // Verify that the compatible prop matches
//                                if prop.name().ok()? == "compatible" && prop.get_str().ok()? == string {
//                                    return Some(prop);
//                                }
//                                None
//
//                            }).and_then(|compatible_prop| {
//                                // If we found a compatible property match, return the node.
//                                return Some(compatible_prop.node());
//                            })
//                        })
//                    }
//            }
//        }
//    }
//}

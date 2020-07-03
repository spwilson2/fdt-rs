use core::alloc::Layout;
use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ptr::{null, null_mut};
use core::str::from_utf8;

use unsafe_unwrap::UnsafeUnwrap;

use crate::base::item::DevTreeItem;
use crate::base::iters::{DevTreeIter, FindNext};
use crate::base::parse::{DevTreeParseIter, ParsedBeginNode, ParsedProp, ParsedTok};
use crate::base::DevTree;
use crate::error::DevTreeError;
use crate::prelude::*;
//use super::item::DevTreeIndexItem;
use super::{DevTreeIndex, DevTreeIndexItem, DevTreeIndexNode, DevTreeIndexProp};
use super::tree::{DTIProp, DTINode};

/***********************************/
/***********  DFS  *****************/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexNodeIter<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node: Option<&'a DTINode<'i, 'dt>>,
}

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexNodeIter<'a, 'i, 'dt> {
    pub(super) fn new(index: &'a DevTreeIndex<'i, 'dt>) -> Self {
        unsafe {
            let root_ref = index.root.as_ref().unsafe_unwrap();

            Self {
                index,
                node: Some(root_ref),
            }
        }
    }
}

impl FindNext for DevTreeIndexNodeIter<'_, '_, '_> {}
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexNodeIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexNode<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.node {
            let cur = DevTreeIndexNode {
                index: self.index,
                node,
            };
            self.node = if let Some(next) = node.first_child() {
                Some(next)
            } else if let Some(next) = node.next() {
                Some(next)
            } else {
                None
            };

            return Some(cur);
        }
        None
    }
}

/***********************************/
/***********  Node Siblings  *******/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexNodeSiblingIter<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node: Option<&'a DTINode<'i, 'dt>>,
}

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexNodeSiblingIter<'a, 'i, 'dt> {
    pub(super) fn new(node: &'a DevTreeIndexNode<'a, 'i, 'dt>) -> Self {
        Self {
            index: &node.index,
            node: Some(node.node),
        }
    }
}

impl FindNext for DevTreeIndexNodeSiblingIter<'_, '_, '_> {}
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexNodeSiblingIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexNode<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.node {
            let cur = DevTreeIndexNode::new(self.index, node);
            self.node = node.next_sibling();
            return Some(cur);
        }
        None
    }
}

/***********************************/
/***********  Node Props ***********/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexNodePropIter<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node: &'a DTINode<'i, 'dt>,
    prop_idx: usize,
}

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexNodePropIter<'a, 'i, 'dt> {
    pub(super) fn new(index: &'a DevTreeIndex<'i, 'dt>, node: &'a DTINode<'i, 'dt>) -> Self {
        Self {
            index,
            node,
            prop_idx: 0,
        }
    }
}

impl FindNext for DevTreeIndexNodePropIter<'_, '_, '_> {}
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexNodePropIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexProp<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.prop_idx < self.node.num_props {
            // Unsafe OK, we just checked the length of props.
            let prop = unsafe { self.node.prop_unchecked(self.prop_idx) };

            self.prop_idx += 1;
            return Some(DevTreeIndexProp::new(self.index, self.node, prop));
        }
        None
    }
}

/***********************************/
/***********  Props      ***********/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexPropIter<'a, 'i: 'a, 'dt: 'i> (DevTreeIndexItemIter<'a, 'i, 'dt>);
impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexPropIter<'a, 'i, 'dt> {
    pub(super) fn new(index: &'a DevTreeIndex<'i, 'dt>) -> Self {
        Self(DevTreeIndexItemIter::new(index))
    }
}

impl FindNext for DevTreeIndexPropIter<'_, '_, '_> {}
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexPropIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexProp<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        for item in &mut self.0 {
            match item {
                DevTreeIndexItem::Node(_) => continue,
                DevTreeIndexItem::Prop(p) => return Some(p),
            }
        }
        None
    }
}

/***********************************/
/***********  Items      ***********/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexItemIter<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node_iter: DevTreeIndexNodeIter<'a, 'i, 'dt>,
    prop_iter: Option<DevTreeIndexNodePropIter<'a, 'i, 'dt>>,
}
impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexItemIter<'a, 'i, 'dt> {
    pub(super) fn new(index: &'a DevTreeIndex<'i, 'dt>) -> Self {
        Self {
            index,
            node_iter: index.nodes(),
            prop_iter: None,
        }
    }
}

impl FindNext for DevTreeIndexItemIter<'_, '_, '_> {}
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexItemIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexItem<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        // - Advance through each node:
        //   - Return than node
        //     - Advance through each prop
        //       - Return that prop
        loop {
            let prop_iter;

            loop {
                if let Some(prop) = &mut self.prop_iter {
                    prop_iter = prop;
                    break;
                }

                match self.node_iter.next() {
                    Some(node) => {
                        self.prop_iter = Some(node.props());
                        return Some(DevTreeIndexItem::Node(node));
                    }
                    None => return None,
                }
            }

            if let Some(prop) = prop_iter.next() {
                return Some(DevTreeIndexItem::Prop(prop));
            }
        }
    }
}

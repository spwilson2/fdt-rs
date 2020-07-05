use crate::prelude::*;

use super::tree::DTINode;
use super::{DevTreeIndex, DevTreeIndexItem, DevTreeIndexNode, DevTreeIndexProp};
use crate::common::iter::{TreeCompatibleNodeIter, TreeNodeIter, TreeNodePropIter, TreePropIter};

/***********************************/
/***********  Node Siblings  *******/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexNodeSiblingIter<'a, 'i: 'a, 'dt: 'i>(DevTreeIndexIter<'a, 'i, 'dt>);

impl<'a, 'i: 'a, 'dt: 'i> From<DevTreeIndexIter<'a, 'i, 'dt>>
    for DevTreeIndexNodeSiblingIter<'a, 'i, 'dt>
{
    fn from(iter: DevTreeIndexIter<'a, 'i, 'dt>) -> Self {
        Self(iter)
    }
}

impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexNodeSiblingIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexNode<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_sibling()
    }
}

pub type DevTreeIndexNodePropIter<'a, 'i, 'dt> =
    TreeNodePropIter<'a, 'dt, DevTreeIndexIter<'a, 'i, 'dt>, DevTreeIndexItem<'a, 'i, 'dt>>;

pub type DevTreeIndexNodeIter<'a, 'i, 'dt> =
    TreeNodeIter<'a, 'dt, DevTreeIndexIter<'a, 'i, 'dt>, DevTreeIndexItem<'a, 'i, 'dt>>;

pub type DevTreeIndexPropIter<'a, 'i, 'dt> =
    TreePropIter<'a, 'dt, DevTreeIndexIter<'a, 'i, 'dt>, DevTreeIndexItem<'a, 'i, 'dt>>;

pub type DevTreeIndexCompatibleNodeIter<'s, 'a, 'i, 'dt> =
    TreeCompatibleNodeIter<'s, 'a, 'dt, DevTreeIndexIter<'a, 'i, 'dt>, DevTreeIndexItem<'a, 'i, 'dt>>;

/***********************************/
/***********  Items      ***********/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexIter<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node: Option<&'a DTINode<'i, 'dt>>,
    prop_idx: usize,
    initial_node_returned: bool,
}

impl<'a, 'i: 'a, 'dt: 'i> TreeIterator<'a, 'dt, DevTreeIndexItem<'a, 'i, 'dt>>
    for DevTreeIndexIter<'a, 'i, 'dt>
{
}

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexIter<'a, 'i, 'dt> {
    pub(super) fn new(index: &'a DevTreeIndex<'i, 'dt>) -> Self {
        let mut this = Self::from_node(index.root());
        this.initial_node_returned = false;
        this
    }

    pub fn from_node(node: DevTreeIndexNode<'a, 'i, 'dt>) -> Self {
        Self {
            index: node.index(),
            initial_node_returned: true,
            node: Some(node.node),
            prop_idx: 0,
        }
    }

    pub fn next_sibling(&mut self) -> Option<DevTreeIndexNode<'a, 'i, 'dt>> {
        self.node.map(|node| {
            let cur = DevTreeIndexNode::new(self.index, node);
            self.node = node.next_sibling();
            cur
        })
    }

    fn next_devtree_item(&mut self) -> Option<DevTreeIndexItem<'a, 'i, 'dt>> {
        self.node.and_then(|cur_node| {
            // Check if we've returned the first current node.
            if !self.initial_node_returned {
                self.initial_node_returned = true;
                return Some(DevTreeIndexItem::Node(DevTreeIndexNode::new(
                    self.index, cur_node,
                )));
            }

            // First iterate through any properties if there are some available.
            if self.prop_idx < cur_node.num_props {
                // Unsafe OK, we just checked the length of props.
                let prop = unsafe { cur_node.prop_unchecked(self.prop_idx) };

                self.prop_idx += 1;
                return Some(DevTreeIndexItem::Prop(DevTreeIndexProp::new(
                    self.index, &cur_node, prop,
                )));
            }

            self.prop_idx = 0;

            // Otherwise move on to the next node.
            self.node = cur_node.next_dfs();
            self.node
                .map(|cur_node| DevTreeIndexItem::Node(DevTreeIndexNode::new(self.index, cur_node)))
        })
    }
}

impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexItem<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_devtree_item()
    }
}

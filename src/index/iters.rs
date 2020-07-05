use crate::prelude::*;

//use super::item::DevTreeIndexItem;
use super::tree::DTINode;
use super::{DevTreeIndex, DevTreeIndexItem, DevTreeIndexNode, DevTreeIndexProp};

/***********************************/
/***********  DFS  *****************/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexNodeIter<'a, 'i: 'a, 'dt: 'i>(DevTreeIndexIter<'a, 'i, 'dt>);

impl<'a, 'i: 'a, 'dt: 'i> From<DevTreeIndexIter<'a, 'i, 'dt>>
    for DevTreeIndexNodeIter<'a, 'i, 'dt>
{
    fn from(iter: DevTreeIndexIter<'a, 'i, 'dt>) -> Self {
        Self(iter)
    }
}

impl<'a, 'i: 'a, 'dt: 'i> From<DevTreeIndexNodeIter<'a, 'i, 'dt>>
    for DevTreeIndexIter<'a, 'i, 'dt>
{
    fn from(iter: DevTreeIndexNodeIter<'a, 'i, 'dt>) -> Self {
        iter.0
    }
}

impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexNodeIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexNode<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_node()
    }
}

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

/***********************************/
/***********  Node Props ***********/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexNodePropIter<'a, 'i: 'a, 'dt: 'i>(DevTreeIndexIter<'a, 'i, 'dt>);

impl<'a, 'i: 'a, 'dt: 'i> From<DevTreeIndexIter<'a, 'i, 'dt>>
    for DevTreeIndexNodePropIter<'a, 'i, 'dt>
{
    fn from(iter: DevTreeIndexIter<'a, 'i, 'dt>) -> Self {
        Self(iter)
    }
}

impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexNodePropIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexProp<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_node_prop()
    }
}

/***********************************/
/***********  Props      ***********/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexPropIter<'a, 'i: 'a, 'dt: 'i>(DevTreeIndexIter<'a, 'i, 'dt>);

impl<'a, 'i: 'a, 'dt: 'i> From<DevTreeIndexIter<'a, 'i, 'dt>>
    for DevTreeIndexPropIter<'a, 'i, 'dt>
{
    fn from(iter: DevTreeIndexIter<'a, 'i, 'dt>) -> Self {
        Self(iter)
    }
}

impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexPropIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexProp<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_prop()
    }
}

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
    type TreeNodeIter = DevTreeIndexNodeIter<'a, 'i, 'dt>;
    type TreePropIter = DevTreeIndexPropIter<'a, 'i, 'dt>;
}

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexIter<'a, 'i, 'dt> {
    #[inline]
    pub(super) fn new(index: &'a DevTreeIndex<'i, 'dt>) -> Self {
        let mut this = Self::from_node(index.root());
        this.initial_node_returned = false;
        this
    }

    #[inline]
    pub fn from_node(node: DevTreeIndexNode<'a, 'i, 'dt>) -> Self {
        Self {
            index: node.index(),
            initial_node_returned: true,
            node: Some(node.node),
            prop_idx: 0,
        }
    }

    #[inline]
    pub fn next_sibling(&mut self) -> Option<DevTreeIndexNode<'a, 'i, 'dt>> {
        self.node.and_then(|node| {
            let cur = DevTreeIndexNode::new(self.index, node);
            self.node = node.next_sibling();
            Some(cur)
        })
    }
}

impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexItem<'a, 'i, 'dt>;

    // Yes, this is a complex function that would traditionally be questionable to inline.
    //
    // We inline this function because callers of this function may completely ignore return
    // values. Effectively, we want callers to be able to choose if they need a node or a prop.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
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

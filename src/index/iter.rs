use core::alloc::Layout;
use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ptr::{null, null_mut};
use core::str::from_utf8;

use unsafe_unwrap::UnsafeUnwrap;

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
pub struct DevTreeIndexNodeIter<'a, 'i: 'a, 'dt: 'i> (DevTreeIndexIter<'a, 'i, 'dt>);

impl<'a, 'i: 'a, 'dt: 'i> From<DevTreeIndexIter<'a, 'i, 'dt>> for DevTreeIndexNodeIter<'a, 'i, 'dt> {
    fn from(iter: DevTreeIndexIter<'a, 'i, 'dt>) -> Self {
        Self(iter)
    }
}

impl FindNext for DevTreeIndexNodeIter<'_, '_, '_> {}
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
pub struct DevTreeIndexNodeSiblingIter<'a, 'i: 'a, 'dt: 'i> (DevTreeIndexIter<'a, 'i, 'dt>);

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexNodeSiblingIter<'a, 'i, 'dt> {
    pub(super) fn from_node(node: DevTreeIndexNode<'a, 'i, 'dt>) -> Self {
        Self(DevTreeIndexIter::from_node(node))
    }
}

impl<'a, 'i: 'a, 'dt: 'i> From<DevTreeIndexIter<'a, 'i, 'dt>> for DevTreeIndexNodeSiblingIter<'a, 'i, 'dt> {
    fn from(iter: DevTreeIndexIter<'a, 'i, 'dt>) -> Self {
        Self(iter)
    }
}

impl FindNext for DevTreeIndexNodeSiblingIter<'_, '_, '_> {}
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
pub struct DevTreeIndexNodePropIter<'a, 'i: 'a, 'dt: 'i> (DevTreeIndexIter<'a, 'i, 'dt>);

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexNodePropIter<'a, 'i, 'dt> {
    pub(super) fn from_node(node: DevTreeIndexNode<'a, 'i, 'dt>) -> Self {
        Self(DevTreeIndexIter::from_node(node))
    }
}

impl<'a, 'i: 'a, 'dt: 'i> From<DevTreeIndexIter<'a, 'i, 'dt>> for DevTreeIndexNodePropIter<'a, 'i, 'dt> {
    fn from(iter: DevTreeIndexIter<'a, 'i, 'dt>) -> Self {
        Self(iter)
    }
}

impl FindNext for DevTreeIndexNodePropIter<'_, '_, '_> {}
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexNodePropIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexProp<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        for item in &mut self.0 {
            match item {
                DevTreeIndexItem::Node(_) => break,
                DevTreeIndexItem::Prop(p) => return Some(p),
            }
        }
        None
    }
}

/***********************************/
/***********  Props      ***********/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexPropIter<'a, 'i: 'a, 'dt: 'i> (DevTreeIndexIter<'a, 'i, 'dt>);
impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexPropIter<'a, 'i, 'dt> {
    pub(super) fn new(index: &'a DevTreeIndex<'i, 'dt>) -> Self {
        Self(DevTreeIndexIter::new(index))
    }
}

impl<'a, 'i: 'a, 'dt: 'i> From<DevTreeIndexIter<'a, 'i, 'dt>> for DevTreeIndexPropIter<'a, 'i, 'dt> {
    fn from(iter: DevTreeIndexIter<'a, 'i, 'dt>) -> Self {
        Self(iter)
    }
}

impl FindNext for DevTreeIndexPropIter<'_, '_, '_> {}
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
                index: node.index,
                initial_node_returned: true,
                node: Some(node.node),
                prop_idx: 0,
            }
    }

    fn current_node_itr(&self) -> Option<Self> {
        if self.node.is_some() {
            return Some(self.clone())
        }
        None
    }

    #[inline]
    pub fn next_node(&mut self) -> Option<DevTreeIndexNode<'a, 'i, 'dt>> {
        loop {
            match self.next() {
                Some(DevTreeIndexItem::Node(n)) => return Some(n),
                Some(_) => {
                    continue;
                }
                _ => return None,
            }
        }
    }

    #[inline]
    pub fn next_prop(&mut self) -> Option<DevTreeIndexProp<'a, 'i, 'dt>> {
        loop {
            match self.next() {
                Some(DevTreeIndexItem::Prop(p)) => return Some(p),
                Some(DevTreeIndexItem::Node(_)) => continue,
                _ => return None,
            }
        }
    }

    #[inline]
    pub fn next_node_prop(&mut self) -> Option<DevTreeIndexProp<'a, 'i, 'dt>> {
        match self.next() {
            Some(DevTreeIndexItem::Prop(p)) => Some(p),
            // Return if a new node or an EOF.
            _ => None,
        }
    }

    #[inline]
    pub fn find_next_compatible_node(&self, string: &str) -> Option<DevTreeIndexNode<'a, 'i, 'dt>> {
        // Create a clone and turn it into a node iterator
        let mut iter = DevTreeIndexNodeIter::from(self.clone());
        // If there is another node
        if iter.next().is_some() {
            // Iterate through its properties looking for the compatible string.
            let mut iter = DevTreeIndexPropIter::from(iter.0);
            if let Some((compatible_prop, _)) = iter.find_next(|prop| unsafe {
                Ok((prop.name()? == "compatible") && (prop.get_str()? == string))
            }) {
                return Some(compatible_prop.node());
            }
        }
        None
    }

    #[inline]
    pub fn next_sibling(&mut self) -> Option<DevTreeIndexNode<'a, 'i, 'dt>> {
        if let Some(node) = self.node {
            let cur = DevTreeIndexNode::new(self.index, node);
            self.node = node.next_sibling();
            return Some(cur);
        }
        None
    }
}

impl FindNext for DevTreeIndexIter<'_, '_, '_> {}
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexItem<'a, 'i, 'dt>;

    // Yes, this is a complex function that would traditionally be questionable to inline.
    //
    // We inline this function because callers of this function may completely ignore return
    // values. Effectively, we want callers to be able to choose if they need a node or a prop.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(cur_node) = self.node {
            // Check if we've returned the first current node.
            if !self.initial_node_returned {
                self.initial_node_returned = true;
                return Some(DevTreeIndexItem::Node(DevTreeIndexNode::new(self.index, cur_node)));
            }

            // First iterate through any properties if there are some available.
            if self.prop_idx < cur_node.num_props {
                // Unsafe OK, we just checked the length of props.
                let prop = unsafe { cur_node.prop_unchecked(self.prop_idx) };

                self.prop_idx += 1;
                return Some(DevTreeIndexItem::Prop(DevTreeIndexProp::new(self.index, &cur_node, prop)));
            }

            self.prop_idx = 0;

            // Otherwise move on to the next node.
            self.node = cur_node.first_child().or(cur_node.next());
            if let Some(cur_node) = self.node {
                return Some(DevTreeIndexItem::Node(DevTreeIndexNode::new(self.index, cur_node)));
            }
        }
        None
    }
}

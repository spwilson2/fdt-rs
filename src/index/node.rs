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
use super::tree::{DevTreeIndex, DTINode, DTIProp};
use super::iter::{DevTreeIndexNodeSiblingIter, DevTreeIndexNodePropIter};

#[derive(Clone)]
pub struct DevTreeIndexNode<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    pub(super) node: &'a DTINode<'i, 'dt>,
}

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexNode<'a, 'i, 'dt> {
    pub(super) fn new(index: &'a DevTreeIndex<'i, 'dt>, node: &'a DTINode<'i, 'dt>) -> Self {
        Self { node, index }
    }

    pub fn name(&self) -> Result<&'dt str, DevTreeError> {
        from_utf8(self.node.name).map_err(|e| DevTreeError::StrError(e))
    }

    pub fn siblings(&self) -> DevTreeIndexNodeSiblingIter<'_, 'i, 'dt> {
        DevTreeIndexNodeSiblingIter::from_node(self.clone())
    }

    pub fn props(&self) -> DevTreeIndexNodePropIter<'a, 'i, 'dt> {
        let node = DevTreeIndexNode::new(self.index, self.node);
        DevTreeIndexNodePropIter::from_node(node)
    }
}

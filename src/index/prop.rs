use crate::prelude::*;

use crate::base::parse::ParsedProp;
use crate::base::DevTree;

use super::tree::{DTINode, DTIProp, DevTreeIndex};
use super::DevTreeIndexNode;

#[derive(Clone)]
pub struct DevTreeIndexProp<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node: &'a DTINode<'i, 'dt>,
    prop: &'a DTIProp<'dt>,
}

impl<'r, 'a: 'r, 'i: 'a, 'dt: 'i> DevTreeIndexProp<'a, 'i, 'dt> {
    pub(super) fn new(
        index: &'a DevTreeIndex<'i, 'dt>,
        node: &'a DTINode<'i, 'dt>,
        prop: &'a DTIProp<'dt>,
    ) -> Self {
        Self { index, node, prop }
    }
    pub fn node(&self) -> DevTreeIndexNode<'a, 'i, 'dt> {
        DevTreeIndexNode::new(self.index, self.node)
    }
}

impl<'r, 'a: 'r, 'i: 'a, 'dt: 'i> DevTreePropState<'r, 'dt> for DevTreeIndexProp<'a, 'i, 'dt> {}
impl<'r, 'a: 'r, 'i: 'a, 'dt: 'i> DevTreePropStateBase<'r, 'dt> for DevTreeIndexProp<'a, 'i, 'dt> {
    #[inline]
    fn propbuf(&'r self) -> &'dt [u8] {
        self.prop.propbuf
    }

    #[inline]
    fn nameoff(&'r self) -> usize {
        self.prop.nameoff
    }

    #[inline]
    fn fdt(&'r self) -> &'r DevTree<'dt> {
        &self.index.fdt()
    }
}

impl<'dt> From<&ParsedProp<'dt>> for DTIProp<'dt> {
    fn from(prop: &ParsedProp<'dt>) -> Self {
        Self {
            propbuf: prop.prop_buf,
            nameoff: prop.name_offset,
        }
    }
}

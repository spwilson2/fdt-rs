use crate::base::iters::DevTreeIter;
use crate::base::{DevTree, DevTreeNode};
use crate::prelude::*;

/// A handle to a [`DevTreeNode`]'s Device Tree Property
#[derive(Clone)]
pub struct DevTreeProp<'a, 'dt:'a> {
    parent_iter: DevTreeIter<'a, 'dt>,
    propbuf: &'dt [u8],
    nameoff: usize,
}

impl<'r, 'dt: 'r> DevTreePropStateBase<'dt> for DevTreeProp<'r, 'dt> {
    type NodeType = DevTreeNode<'r, 'dt>;

    #[inline]
    fn propbuf(&self) -> &'dt [u8] {
        self.propbuf
    }

    #[inline]
    fn nameoff(&self) -> usize {
        self.nameoff
    }

    #[inline]
    fn fdt(&self) -> &DevTree<'dt> {
        self.parent_iter.fdt
    }

    /// Returns the node which this property is attached to
    #[inline]
    #[must_use]
    fn node(&self) -> DevTreeNode<'r, 'dt> {
        self.parent_iter.clone().next_node().unwrap()
    }
}

impl<'a, 'dt:'a> DevTreeProp<'a, 'dt> {

    pub(super) fn new(parent_iter: DevTreeIter<'a, 'dt>, propbuf: &'dt [u8], nameoff: usize) -> Self {
        Self {
            parent_iter,
            propbuf,
            nameoff,
        }
    }
}

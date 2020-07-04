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

impl<'r, 'dt: 'r> DevTreePropState<'r, 'dt> for DevTreeProp<'r, 'dt> {}
impl<'r, 'dt: 'r> DevTreePropStateBase<'r, 'dt> for DevTreeProp<'r, 'dt> {
    #[inline]
    fn propbuf(&'r self) -> &'dt [u8] {
        self.propbuf
    }

    #[inline]
    fn nameoff(&'r self) -> usize {
        self.nameoff
    }

    #[inline]
    fn fdt(&'r self) -> &'r DevTree<'dt> {
        self.parent_iter.fdt
    }
}

impl<'a, 'dt:'a> DevTreeProp<'a, 'dt> {
    /// Returns the node which this property is attached to
    #[inline]
    #[must_use]
    pub fn node(&self) -> DevTreeNode<'a, 'dt> {
        self.parent_iter.clone().next_node().unwrap()
    }

    pub(super) fn new(parent_iter: DevTreeIter<'a, 'dt>, propbuf: &'dt [u8], nameoff: usize) -> Self {
        Self {
            parent_iter,
            propbuf,
            nameoff,
        }
    }
}

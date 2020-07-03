use crate::base::iters::DevTreeIter;
use crate::base::{DevTree, DevTreeNode};
use crate::prelude::*;

/// A handle to a [`DevTreeNode`]'s Device Tree Property
#[derive(Clone)]
pub struct DevTreeProp<'dt> {
    parent_iter: DevTreeIter<'dt>,
    propbuf: &'dt [u8],
    nameoff: usize,
}

impl<'r, 'dt: 'r> DevTreePropState<'r, 'dt> for DevTreeProp<'dt> {}
impl<'r, 'dt: 'r> DevTreePropStateBase<'r, 'dt> for DevTreeProp<'dt> {
    #[inline]
    fn propbuf(&'r self) -> &'dt [u8] {
        self.propbuf
    }

    #[inline]
    fn nameoff(&'r self) -> usize {
        self.nameoff
    }

    #[inline]
    fn fdt(&'r self) -> &'dt DevTree<'dt> {
        self.parent_iter.fdt
    }
}

impl<'a> DevTreeProp<'a> {
    /// Returns the node which this property is attached to
    #[inline]
    #[must_use]
    pub fn parent(&self) -> DevTreeNode<'a> {
        self.parent_iter.clone().next_node().unwrap()
    }

    pub(super) fn new(parent_iter: DevTreeIter<'a>, propbuf: &'a [u8], nameoff: usize) -> Self {
        Self {
            parent_iter,
            propbuf,
            nameoff,
        }
    }
}

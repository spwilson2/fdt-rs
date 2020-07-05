#[cfg(doc)]
use super::*;

use crate::prelude::*;

use crate::base::iters::{DevTreeIter, DevTreeNodePropIter};
use crate::error::DevTreeError;

/// A handle to a Device Tree Node within the device tree.
#[derive(Clone)]
pub struct DevTreeNode<'a, 'dt: 'a> {
    pub(super) name: Result<&'dt str, DevTreeError>,
    pub(super) parse_iter: DevTreeIter<'a, 'dt>,
}

impl<'a, 'dt: 'a> DevTreeNode<'a, 'dt> {
    /// Returns the name of the `DevTreeNode` (including unit address tag)
    #[inline]
    pub fn name(&'a self) -> Result<&'dt str, DevTreeError> {
        self.name
    }

    /// Returns an iterator over this node's children [`DevTreeProp`]
    #[must_use]
    pub fn props(&'a self) -> DevTreeNodePropIter<'a, 'dt> {
        DevTreeNodePropIter::new(self)
    }

    /// Returns the next [`DevTreeNode`] object with the provided compatible device tree property
    /// or `None` if none exists.
    ///
    /// # Example
    ///
    /// The following example iterates through all nodes with compatible value "virtio,mmio"
    /// and prints each node's name.
    ///
    /// TODO
    pub fn find_next_compatible_node(&self, string: &str) -> Option<DevTreeNode<'a, 'dt>> {
        self.parse_iter.clone().next_compatible_node(string)
    }
}

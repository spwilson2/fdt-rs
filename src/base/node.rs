#[cfg(doc)]
use super::*;

use crate::base::iters::{DevTreeIter, DevTreeNodePropIter};
use crate::error::DevTreeError;

/// A handle to a Device Tree Node within the device tree.
#[derive(Clone)]
pub struct DevTreeNode<'a> {
    pub(super) name: Result<&'a str, DevTreeError>,
    pub(super) parse_iter: DevTreeIter<'a>,
}

impl<'a> DevTreeNode<'a> {
    /// Returns the name of the `DevTreeNode` (including unit address tag)
    #[inline]
    pub fn name(&'a self) -> Result<&'a str, DevTreeError> {
        self.name
    }

    /// Returns an iterator over this node's children [`DevTreeProp`]
    #[inline]
    #[must_use]
    pub fn props(&'a self) -> DevTreeNodePropIter<'a> {
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
    /// ```
    /// # let mut devtree = fdt_rs::doctest::get_devtree();
    /// let compat = "virtio,mmio";
    /// # let mut count = 0;
    /// if let Some(mut cur) = devtree.root() {
    ///     while let Some(node) = cur.find_next_compatible_node(compat) {
    ///         println!("{}", node.name()?);
    ///         # count += 1;
    ///         # assert!(node.name()?.starts_with("virtio_mmio@1000"));
    ///         cur = node;
    ///     }
    /// }
    /// # assert!(count == 8);
    /// # Ok::<(), fdt_rs::DevTreeError>(())
    /// ```
    #[inline]
    pub fn find_next_compatible_node(&self, string: &str) -> Option<DevTreeNode<'a>> {
        self.parse_iter.find_next_compatible_node(string)
    }
}

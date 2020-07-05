use crate::prelude::*;

pub trait IterableDevTree<'a, 'dt: 'a> {
    type TreeNode;
    type TreeIter;
    type NodeIter;
    type PropIter;

    #[must_use]
    fn props(&'a self) -> Self::PropIter;
    #[must_use]
    fn nodes(&'a self) -> Self::NodeIter;
    #[must_use]
    fn items(&'a self) -> Self::TreeIter;
    fn find_first_compatible_node(&'a self, string: &str) -> Option<Self::TreeNode>;
    #[must_use]
    fn buf(&'a self) -> &'dt [u8];
    fn root(&'a self) -> Option<Self::TreeNode>;
}

pub trait TreeIterator<'r, 'dt: 'r, I>: Clone + Iterator<Item = I>
where
    I: UnwrappableDevTreeItem<'dt>,
{
    type TreeNodeIter: From<Self> + Into<Self> + Iterator<Item = I::TreeNode>;
    type TreePropIter: From<Self> + Iterator<Item = I::TreeProp>;

    fn next_prop(&mut self) -> Option<I::TreeProp> {
        loop {
            match self.next() {
                Some(item) => {
                    if let Some(prop) = item.prop() {
                        return Some(prop);
                    }
                    // Continue if a new node.
                    continue;
                }
                _ => return None,
            }
        }
    }

    fn next_node(&mut self) -> Option<I::TreeNode> {
        loop {
            match self.next() {
                Some(item) => {
                    if let Some(node) = item.node() {
                        return Some(node);
                    }
                    // Continue if a new prop.
                    continue;
                }
                _ => return None,
            }
        }
    }

    fn next_node_prop(&mut self) -> Option<I::TreeProp> {
        match self.next() {
            // Return if a new node or an EOF.
            Some(item) => item.prop(),
            _ => None,
        }
    }

    fn find_next_compatible_node(
        &self,
        string: &str,
    ) -> Option<<I::TreeProp as PropReaderBase<'dt>>::NodeType> {
        // Create a clone and turn it into a node iterator
        let mut node_iter = Self::TreeNodeIter::from(self.clone());

        // If there is another node, advance our iterator to that node.
        node_iter.next().and_then(|_| {
            // Iterate through all remaining properties in the tree looking for the compatible
            // string.
            let mut iter = Self::TreePropIter::from(node_iter.into());
            iter.find_map(|prop| unsafe {
                // Verify that the compatible prop matches
                if prop.name().ok()? == "compatible" && prop.get_str().ok()? == string {
                    return Some(prop);
                }
                None
            })
            .and_then(|compatible_prop| {
                // If we found a compatible property match, return the node.
                return Some(compatible_prop.node());
            })
        })
    }
}

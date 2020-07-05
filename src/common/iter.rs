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

    fn next_compatible_node(
        &mut self,
        string: &str,
    ) -> Option<<I::TreeProp as PropReaderBase<'dt>>::NodeType> {
        // If there is another node, advance our iterator to that node.
        self.next_node().and_then(|_| {
            // Iterate through all remaining properties in the tree looking for the compatible
            // string.
            while let Some(prop) = self.next_prop() {
                unsafe {
                    if prop.name().ok()? == "compatible" && prop.get_str().ok()? == string {
                        return Some(prop.node())
                    }
                }
            }
            None
        })
    }
}

use crate::prelude::*;
use core::marker::PhantomData;

// Shared trait for common tree iterators
pub trait IterableDevTree<'s, 'a, 'dt: 'a> {
    type TreeNode;
    type TreeIter;
    type NodeIter;
    type PropIter;
    type CompatibleIter;

    #[must_use]
    fn props(&'a self) -> Self::PropIter;
    #[must_use]
    fn nodes(&'a self) -> Self::NodeIter;
    #[must_use]
    fn items(&'a self) -> Self::TreeIter;
    fn compatible_nodes(&'a self, string: &'s str) -> Self::CompatibleIter;
    #[must_use]
    fn buf(&'a self) -> &'dt [u8];
    fn root(&'a self) -> Option<Self::TreeNode>;
}

/*************************************/
/**         Property Iterator       **/

#[derive(Clone)]
pub struct TreePropIter<'r, 'dt: 'r, II, I>(II, PhantomData<&'dt ()>, PhantomData<&'r I>)
where
    II: TreeIterator<'r, 'dt, I> + Clone,
    I: UnwrappableDevTreeItem<'dt>;

impl<'r, 'dt: 'r, II, I> From<II> for TreePropIter<'r, 'dt, II, I>
where
    II: TreeIterator<'r, 'dt, I> + Clone,
    I: UnwrappableDevTreeItem<'dt>,
{
    fn from(iter: II) -> Self {
        Self(iter, PhantomData, PhantomData)
    }
}
impl<'r, 'dt: 'r, II, I> Iterator for TreePropIter<'r, 'dt, II, I>
where
    II: TreeIterator<'r, 'dt, I> + Clone,
    I: UnwrappableDevTreeItem<'dt>,
{
    type Item = I::TreeProp;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_prop()
    }
}

/*************************************/
/**           Node Iterator         **/

#[derive(Clone)]
pub struct TreeNodeIter<'r, 'dt: 'r, II, I>(II, PhantomData<&'dt ()>, PhantomData<&'r I>)
where
    II: TreeIterator<'r, 'dt, I> + Clone,
    I: UnwrappableDevTreeItem<'dt>;

impl<'r, 'dt: 'r, II, I> From<II> for TreeNodeIter<'r, 'dt, II, I>
where
    II: TreeIterator<'r, 'dt, I> + Clone,
    I: UnwrappableDevTreeItem<'dt>,
{
    fn from(iter: II) -> Self {
        Self(iter, PhantomData, PhantomData)
    }
}
impl<'r, 'dt: 'r, II, I> Iterator for TreeNodeIter<'r, 'dt, II, I>
where
    II: TreeIterator<'r, 'dt, I> + Clone,
    I: UnwrappableDevTreeItem<'dt>,
{
    type Item = I::TreeNode;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_node()
    }
}

/*************************************/
/**         Node Prop Iterator      **/

#[derive(Clone)]
pub struct TreeNodePropIter<'r, 'dt: 'r, II, I>(II, PhantomData<&'dt ()>, PhantomData<&'r I>)
where
    II: TreeIterator<'r, 'dt, I> + Clone,
    I: UnwrappableDevTreeItem<'dt>;

impl<'r, 'dt: 'r, II, I> From<II> for TreeNodePropIter<'r, 'dt, II, I>
where
    II: TreeIterator<'r, 'dt, I> + Clone,
    I: UnwrappableDevTreeItem<'dt>,
{
    fn from(iter: II) -> Self {
        Self(iter, PhantomData, PhantomData)
    }
}

impl<'r, 'dt: 'r, II, I> Iterator for TreeNodePropIter<'r, 'dt, II, I>
where
    II: TreeIterator<'r, 'dt, I> + Clone,
    I: UnwrappableDevTreeItem<'dt>,
{
    type Item = I::TreeProp;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_node_prop()
    }
}

/********************************************/
/**         Compatible Node Iterator      **/

#[derive(Clone)]
pub struct TreeCompatibleNodeIter<'s, 'r, 'dt: 'r, II, I>(
    II,
    &'s str,
    PhantomData<&'dt ()>,
    PhantomData<&'r I>,
)
where
    II: TreeIterator<'r, 'dt, I> + Clone,
    I: UnwrappableDevTreeItem<'dt>;

impl<'s, 'r, 'dt: 'r, II, I> TreeCompatibleNodeIter<'s, 'r, 'dt, II, I>
where
    II: TreeIterator<'r, 'dt, I> + Clone,
    I: UnwrappableDevTreeItem<'dt>,
{
    pub fn new(iter: II, string: &'s str) -> Self {
        Self(iter, string, PhantomData, PhantomData)
    }
}

impl<'s, 'r, 'dt: 'r, II, I> Iterator for TreeCompatibleNodeIter<'s, 'r, 'dt, II, I>
where
    II: TreeIterator<'r, 'dt, I> + Clone,
    I: UnwrappableDevTreeItem<'dt>,
{
    type Item = <I::TreeProp as crate::common::prop::PropReader<'dt>>::NodeType;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next_compatible_node(self.1)
    }
}

/*********************************************/
/**     Generic Item Iter Implementation    **/

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
    ) -> Option<<I::TreeProp as PropReader<'dt>>::NodeType> {
        // If there is another node, advance our iterator to that node.
        self.next_node().and_then(|_| {
            // Iterate through all remaining properties in the tree looking for the compatible
            // string.
            while let Some(prop) = self.next_prop() {
                unsafe {
                    if prop.name().ok()? == "compatible" && prop.get_str().ok()? == string {
                        return Some(prop.node());
                    }
                }
            }
            None
        })
    }
}

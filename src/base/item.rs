use crate::base::{DevTreeNode, DevTreeProp};

/// An enum which contains either a [`DevTreeNode`] or a [`DevTreeProp`]
#[derive(Clone)]
pub enum DevTreeItem<'a, 'dt:'a> {
    Node(DevTreeNode<'a, 'dt>),
    Prop(DevTreeProp<'a, 'dt>),
}

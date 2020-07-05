use crate::base::{DevTreeNode, DevTreeProp};

/// An enum which contains either a [`DevTreeNode`] or a [`DevTreeProp`]
#[derive(Clone)]
pub enum DevTreeItem<'a, 'dt:'a> {
    Node(DevTreeNode<'a, 'dt>),
    Prop(DevTreeProp<'a, 'dt>),
}

pub trait UnwrappableDevTreeItem { 
    type TreeNode;
    type TreeProp;

    fn consume(self) -> (Option<Self::TreeNode>, Option<Self::TreeProp>);
    fn node(self) -> Option<Self::TreeNode>;
    fn prop(self) -> Option<Self::TreeProp>;
}

impl<'a, 'dt:'a> UnwrappableDevTreeItem for DevTreeItem<'a, 'dt> {
    type TreeNode = DevTreeNode<'a, 'dt>;
    type TreeProp = DevTreeProp<'a, 'dt>;

    #[inline]
    fn consume(self) -> (Option<Self::TreeNode>, Option<Self::TreeProp>) {
        match self {
            DevTreeItem::Node(n) => (Some(n),None),
            DevTreeItem::Prop(p) => (None,Some(p)),
        }
    }

    #[inline]
    fn node(self) -> Option<Self::TreeNode> {
        match self {
            DevTreeItem::Node(node) => Some(node),
            _ => None,
        }
    }

    #[inline]
    fn prop(self) -> Option<Self::TreeProp> {
        match self {
            DevTreeItem::Prop(prop) => Some(prop),
            _ => None,
        }
    }
}

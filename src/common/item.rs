use crate::prelude::*;

pub trait UnwrappableDevTreeItem<'dt> {
    type TreeNode;
    type TreeProp: PropReader<'dt>;
    fn node(self) -> Option<Self::TreeNode>;
    fn prop(self) -> Option<Self::TreeProp>;
}

use super::{DevTreeIndexProp, DevTreeIndexNode};

#[derive(Clone)]
pub enum DevTreeIndexItem<'a, 'i: 'a, 'dt: 'i> {
    Node(DevTreeIndexNode<'a, 'i, 'dt>),
    Prop(DevTreeIndexProp<'a, 'i, 'dt>),
}

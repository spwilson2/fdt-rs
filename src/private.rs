use crate::DevTree;

pub trait DevTreePropStateBase<'r, 'dt: 'r> {
    fn propbuf(&'r self) -> &'dt [u8];
    fn nameoff(&'r self) -> usize;
    fn fdt(&'r self) -> &'r DevTree<'dt>;
}

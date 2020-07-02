use crate::*;
use crate::iters::AssociatedOffset;
pub trait DevTreePropStateBase<'r, 'dt: 'r> {
    fn propbuf(&'r self) -> &'dt [u8];
    fn nameoff(&'r self) -> AssociatedOffset<'dt>;
    fn fdt(&'r self) -> &'r DevTree<'dt>;
}

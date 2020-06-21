use crate::*;
use crate::iters::AssociatedOffset;
pub trait DevTreePropStateBase<'dt> {
    fn propbuf(&self) -> &'dt [u8];
    fn nameoff(&self) -> AssociatedOffset<'dt>;
    fn fdt(&self) -> &'dt DevTree<'dt>;
}

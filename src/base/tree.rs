#[cfg(doc)]
use crate::base::*;

use crate::prelude::*;

use core::mem::size_of;

use crate::error::DevTreeError;

use crate::priv_util::SliceRead;
use crate::spec::{fdt_header, FDT_MAGIC};

use super::iters::{DevTreeIter, DevTreeNodeIter, DevTreePropIter, DevTreeReserveEntryIter, DevTreeCompatibleNodeIter};
use super::DevTreeNode;

const fn is_aligned<T>(offset: usize) -> bool {
    offset % size_of::<T>() == 0
}

const fn verify_offset_aligned<T>(offset: usize) -> Result<usize, DevTreeError> {
    let i: [Result<usize, DevTreeError>; 2] = [Err(DevTreeError::ParseError), Ok(offset)];
    i[is_aligned::<T>(offset) as usize]
}

macro_rules! get_be32_field {
    ( $f:ident, $s:ident , $buf:expr ) => {
        $buf.read_be_u32(offset_of!($s, $f))
    };
}

/// A parseable Flattened Device Tree.
///
/// This parser was written according to the v0.3 specification provided at
/// https://www.devicetree.org/
#[derive(Copy, Clone, Debug)]
pub struct DevTree<'dt> {
    buf: &'dt [u8],
}

impl<'dt> DevTree<'dt> {
    pub const MIN_HEADER_SIZE: usize = size_of::<fdt_header>();
    /// Verify the magic header of a Device Tree buffer
    ///
    /// # Safety
    ///
    /// Callers of this method the must guarantee the following:
    /// - The passed buffer is 32-bit aligned.
    ///
    /// The passed byte buffer will be interpreted as a Flattened Device Tree. For this reason this API
    /// is marked unsafe.
    #[inline]
    pub unsafe fn verify_magic(buf: &[u8]) -> Result<(), DevTreeError> {
        if get_be32_field!(magic, fdt_header, buf)? != FDT_MAGIC {
            Err(DevTreeError::InvalidMagicNumber)
        } else {
            Ok(())
        }
    }

    /// Using the provided byte slice this method will:
    ///
    /// 1. Verify that the slice begins with the magic Device Tree header
    /// 2. Return the reported `totalsize` field of the Device Tree header
    ///
    /// When one must parse a Flattened Device Tree, it's possible that the actual size of the device
    /// tree may be unknown. For that reason, this method can be called before constructing the
    /// [`DevTree`].
    ///
    /// Once known, the user should resize the raw byte slice to this function's return value and
    /// pass that slice to [`DevTree::new()`].
    ///
    /// # Example
    ///
    /// TODO
    ///
    /// # Safety
    ///
    /// Callers of this method the must guarantee the following:
    /// - The passed buffer is 32-bit aligned.
    /// - The passed buffer is of at least [`DevTree::MIN_HEADER_SIZE`] bytes in length
    ///
    /// The passed byte buffer will be interpreted as a Flattened Device Tree. For this reason this API
    /// is marked unsafe.
    #[inline]
    pub unsafe fn read_totalsize(buf: &[u8]) -> Result<usize, DevTreeError> {
        // Verify provided buffer alignment
        verify_offset_aligned::<u32>(buf.as_ptr() as usize)
            .map_err(|_| DevTreeError::InvalidParamter("Unaligned buffer provided"))?;

        // Verify provided buffer magic
        Self::verify_magic(buf)?;
        Ok(get_be32_field!(totalsize, fdt_header, buf)? as usize)
    }

    /// Construct the parseable DevTree object from the provided byte slice.
    ///
    /// # Safety
    ///
    /// Callers of this method the must guarantee the following:
    /// - The passed buffer is 32-bit aligned.
    /// - The passed buffer is exactly the length returned by `Self::read_totalsize()`
    ///
    ///
    #[inline]
    pub unsafe fn new(buf: &'dt [u8]) -> Result<Self, DevTreeError> {
        if Self::read_totalsize(buf)? < buf.len() {
            Err(DevTreeError::ParseError)
        } else {
            let ret = Self { buf };
            // Verify required alignment before returning.
            verify_offset_aligned::<u32>(ret.off_mem_rsvmap())?;
            verify_offset_aligned::<u32>(ret.off_dt_struct())?;
            Ok(ret)
        }
    }

    /// Returns the totalsize field of the Device Tree
    #[inline]
    #[must_use]
    pub fn totalsize(&self) -> usize {
        unsafe { get_be32_field!(totalsize, fdt_header, self.buf).unwrap() as usize }
    }

    /// Returns the of rsvmap offset field of the Device Tree
    #[inline]
    #[must_use]
    pub fn off_mem_rsvmap(&self) -> usize {
        unsafe { get_be32_field!(off_mem_rsvmap, fdt_header, self.buf).unwrap() as usize }
    }

    /// Returns the of dt_struct offset field of the Device Tree
    #[inline]
    #[must_use]
    pub fn off_dt_struct(&self) -> usize {
        unsafe { get_be32_field!(off_dt_struct, fdt_header, self.buf).unwrap() as usize }
    }

    /// Returns the of dt_strings offset field of the Device Tree
    #[inline]
    #[must_use]
    pub fn off_dt_strings(&self) -> usize {
        unsafe { get_be32_field!(off_dt_strings, fdt_header, self.buf).unwrap() as usize }
    }

    /// Returns a typed `*const T` to the given offset in the Device Tree buffer.
    ///
    /// # Safety
    ///
    /// Due to the unsafe nature of re-interpretation casts this method is unsafe.  This method
    /// will verify that enough space to fit type T remains within the buffer.
    ///
    /// The caller must verify that the pointer is not misaligned before it is dereferenced.
    pub(crate) unsafe fn ptr_at<T>(&self, offset: usize) -> Result<*const T, DevTreeError> {
        if offset + size_of::<T>() > self.buf.len() {
            Err(DevTreeError::InvalidOffset)
        } else {
            Ok(self.buf.as_ptr().add(offset) as *const T)
        }
    }

    /// Returns an iterator over the Dev Tree "5.3 Memory Reservation Blocks"
    #[must_use]
    pub fn reserved_entries(&self) -> DevTreeReserveEntryIter {
        DevTreeReserveEntryIter::new(self)
    }
}

impl<'s, 'a, 'dt: 'a> IterableDevTree<'s, 'a, 'dt> for DevTree<'dt> {
    type TreeNode = DevTreeNode<'a, 'dt>;
    type TreeIter = DevTreeIter<'a, 'dt>;
    type NodeIter = DevTreeNodeIter<'a, 'dt>;
    type PropIter = DevTreePropIter<'a, 'dt>;
    type CompatibleIter = DevTreeCompatibleNodeIter<'s, 'a, 'dt>;

    /// Returns an iterator over [`DevTreeNode`] objects
    fn nodes(&'a self) -> Self::NodeIter {
        DevTreeNodeIter::from(DevTreeIter::new(self))
    }

    #[must_use]
    fn props(&'a self) -> Self::PropIter {
        DevTreePropIter::from(DevTreeIter::new(self))
    }

    /// Returns an iterator over objects within the [`DevTreeItem`] enum
    fn items(&'a self) -> Self::TreeIter {
        DevTreeIter::new(self)
    }

    /// Returns the first [`DevTreeNode`] object with the provided compatible device tree property
    /// or `None` if none exists.
    fn compatible_nodes(&'a self, string: &'s str) -> Self::CompatibleIter {
        DevTreeCompatibleNodeIter::new(self.items(), string)
    }

    fn buf(&'a self) -> &'dt [u8] {
        self.buf
    }

    /// Returns the root [`DevTreeNode`] object of the device tree (if it exists).
    fn root(&self) -> Option<DevTreeNode<'_, 'dt>> {
        self.nodes().next()
    }
}

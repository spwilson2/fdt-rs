use core::mem::size_of;

use crate::error::DevTreeError;
use crate::prelude::*;
use crate::priv_util::SliceRead;
use crate::spec::{fdt_header, FDT_MAGIC};

use super::iters::{DevTreeIter, DevTreeNodeIter, DevTreePropIter, DevTreeReserveEntryIter};
use super::{DevTreeItem, DevTreeNode, DevTreeProp};

#[inline]
const fn is_aligned<T>(offset: usize) -> bool {
    offset % size_of::<T>() == 0
}

#[inline]
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
pub struct DevTree<'a> {
    buf: &'a [u8],
}

impl<'a> DevTree<'a> {
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
    /// ```
    /// # use fdt_rs::*;
    /// # let buf = fdt_rs::doctest::FDT;
    /// // Data is re-interpreted as a device tree, this is unsafe.
    /// // See safety section
    /// unsafe {
    ///     let size = DevTree::read_totalsize(buf)?;
    ///     let buf = &buf[..size];
    ///     let dt = DevTree::new(buf)?;
    /// }
    /// # Ok::<(), fdt_rs::DevTreeError>(())
    /// ```
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
        assert!(
            verify_offset_aligned::<u32>(buf.as_ptr() as usize).is_ok(),
            "Unaligned buffer provided"
        );
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
    pub unsafe fn new(buf: &'a [u8]) -> Result<Self, DevTreeError> {
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
    #[inline]
    pub(crate) unsafe fn ptr_at<T>(&self, offset: usize) -> Result<*const T, DevTreeError> {
        if offset + size_of::<T>() > self.buf.len() {
            Err(DevTreeError::InvalidOffset)
        } else {
            Ok(self.buf.as_ptr().add(offset) as *const T)
        }
    }

    /// Returns an iterator over the Dev Tree "5.3 Memory Reservation Blocks"
    #[inline]
    #[must_use]
    pub fn reserved_entries(&self) -> DevTreeReserveEntryIter {
        DevTreeReserveEntryIter::new(self)
    }

    /// Returns an iterator over [`DevTreeNode`] objects
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> DevTreeNodeIter {
        DevTreeNodeIter::new(self)
    }

    /// Returns an iterator over objects within the [`DevTreeItem`] enum
    #[inline]
    #[must_use]
    pub fn items(&self) -> DevTreeIter {
        DevTreeIter::new(self)
    }

    /// Returns the root [`DevTreeNode`] object of the device tree (if it exists).
    #[inline]
    pub fn root(&self) -> Option<DevTreeNode> {
        self.nodes().next()
    }

    /// Map the supplied predicate over the [`DevTreeItem`] enum.
    ///
    /// If the predicate returns `true`, Some(([`DevTreeItem`], [`DevTreeIter`])) will be returned.
    /// The [`DevTreeIter`] may be used to continue searching through the tree.
    ///
    /// The predicate function may return true to simply terminate the search.
    ///
    /// # Example
    ///
    /// ```
    /// # use fdt_rs::*;
    /// # let mut devtree = fdt_rs::doctest::get_devtree();
    /// fn is_uart_compatible(item: &DevTreeItem) -> Result<bool, DevTreeError> {
    ///     unsafe {
    ///         match item {
    ///             DevTreeItem::Prop(p) => {
    ///                 Ok((p.name()? == "compatible") && (p.get_str()? == "ns16550a"))
    ///             },
    ///             _ => Ok(false),
    ///         }
    ///     }
    /// }
    ///
    /// // Print the names of all compatible uarts
    /// if let Some((DevTreeItem::Prop(compatible_prop), mut iter)) = devtree.find(is_uart_compatible) {
    ///     println!("{}", compatible_prop.parent().name()?);
    ///     # assert!(compatible_prop.parent().name()? == "uart@10000000");
    ///
    ///     // Continue the search and keep printing their names.
    ///     while let Some((DevTreeItem::Prop(compatible_prop), mut iter)) = iter.find_next(is_uart_compatible) {
    ///         # assert!(false, "Found uart node that should not have existed.");
    ///         println!("{}", compatible_prop.parent().name()?);
    ///     }
    /// }
    /// # Ok::<(), fdt_rs::DevTreeError>(())
    /// ```
    ///
    #[inline]
    pub fn find<F>(&'a self, predicate: F) -> Option<(DevTreeItem<'a>, DevTreeIter<'a>)>
    where
        F: Fn(&DevTreeItem) -> Result<bool, DevTreeError>,
    {
        DevTreeIter::new(self).find_next(predicate)
    }

    /// Map the supplied predicate over all [`DevTreeProp`] objects
    ///
    /// If the predicate returns `true`, Some(([`DevTreeProp`], [`DevTreePropIter`])) will be returned.
    /// The [`DevTreePropIter`] may be used to continue searching through the tree.
    ///
    /// # Example
    ///
    /// ```
    /// # let mut devtree = fdt_rs::doctest::get_devtree();
    /// // Print the first "ns16550a" compatible node.
    /// if let Some((compatible_prop, _)) = devtree.find_prop(|prop| unsafe {
    ///     Ok((prop.name()? == "compatible") && (prop.get_str()? == "ns16550a"))
    ///     }) {
    ///     println!("{}", compatible_prop.parent().name()?);
    ///     # assert!(compatible_prop.parent().name()? == "uart@10000000");
    /// }
    /// # Ok::<(), fdt_rs::DevTreeError>(())
    /// ```
    ///
    #[inline]
    pub fn find_prop<F>(&'a self, predicate: F) -> Option<(DevTreeProp<'a>, DevTreePropIter<'a>)>
    where
        F: Fn(&DevTreeProp) -> Result<bool, DevTreeError>,
    {
        DevTreePropIter::new(self).find_next(predicate)
    }

    /// Map the supplied predicate over all [`DevTreeNode`] objects
    ///
    /// If the predicate returns `true`, Some(([`DevTreeItem`], [`DevTreeNodeIter`])) will be returned.
    /// The [`DevTreeNodeIter`] may be used to continue searching through the tree.
    #[inline]
    pub fn find_node<F>(&'a self, predicate: F) -> Option<(DevTreeNode<'a>, DevTreeNodeIter<'a>)>
    where
        F: Fn(&DevTreeNode) -> Result<bool, DevTreeError>,
    {
        DevTreeNodeIter::new(self).find_next(predicate)
    }

    /// Returns the first [`DevTreeNode`] object with the provided compatible device tree property
    /// or `None` if none exists.
    #[inline]
    pub fn find_first_compatible_node(&'a self, string: &str) -> Option<DevTreeNode<'a>> {
        self.items().find_next_compatible_node(string)
    }

    #[inline]
    pub fn buf(&self) -> &'a [u8] {
        self.buf
    }
}

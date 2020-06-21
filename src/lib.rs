//! A flattened device tree parser for embedded, low memory, and safety-critical no-std environment
//!
//! * This device tree parser uses zero-allocation
//! * Remains safe even in the event of an invalid device tree
//! * Never performs misaligned reads
//!
//! ## Features
//!
//! This crate can be used without the standard library (`#![no_std]`) by disabling
//! the default `std` feature. To use `no-std` place the following in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies.fdt-rs]
//! version = "0.1"
//! default-features = false
//! # features = ["ascii"]    # <--- Uncomment if you wish to use the ascii crate for str's
//! ```
//!
//! Embeded software may not require the use of utf8 strings. For memory and processing constrained
//! environments ASCII may be suitable. For this reason, this crate supports the use of either
//! ascii or standard rust utf-8 `str`  types.
//!
//! Enabling the `"ascii"` feature will configure the `Str` type returned by string accessor
//! methods to be of type `AsciiStr` provided by the
//! [ascii crate](https://docs.rs/ascii/1.0.0/ascii/).
//!
#![deny(clippy::all, clippy::cargo)]
#![allow(clippy::as_conversions)]
// Test the readme if using nightly.
#![cfg_attr(RUSTC_IS_NIGHTLY, feature(external_doc))]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "std")]
extern crate core;
#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;
#[macro_use]
extern crate cfg_if;
extern crate endian_type_rs as endian_type;
#[macro_use]
extern crate memoffset;
extern crate unsafe_unwrap;

mod buf_util;
pub mod iters;
pub mod spec;

#[cfg(any(feature = "std", feature = "alloc"))]
pub mod index;
pub(crate) mod fdt_util;

use core::convert::From;
use core::mem::size_of;

use buf_util::{SliceRead, SliceReadError};
use spec::{fdt_header, Phandle, FDT_MAGIC};
use fdt_util::props::DevTreePropState;

cfg_if! {
    if #[cfg(feature = "ascii")] {
        extern crate ascii;

        pub type StrError = ascii::AsAsciiStrError;
        pub type Str = ascii::AsciiStr;
        fn bytes_as_str(buf: &[u8]) -> Result<& Str, StrError> {
            ascii::AsciiStr::from_ascii(buf)
        }
    } else {
        pub type StrError = core::str::Utf8Error;
        pub type Str = str;
        fn bytes_as_str(buf: &[u8]) -> Result<& Str, StrError> {
            core::str::from_utf8(buf)
        }
    }
}

macro_rules! get_be32_field {
    ( $f:ident, $s:ident , $buf:expr ) => {
        $buf.read_be_u32(offset_of!($s, $f))
    };
}

#[inline]
const fn is_aligned<T>(offset: usize) -> bool {
    offset % size_of::<T>() == 0
}

#[inline]
const fn verify_offset_aligned<T>(offset: usize) -> Result<usize, DevTreeError> {
    let i: [Result<usize, DevTreeError>; 2] = [Err(DevTreeError::ParseError), Ok(offset)];
    i[is_aligned::<T>(offset) as usize]
}

/// An error describe parsing problems when creating device trees.
#[derive(Debug, Clone, Copy)]
pub enum DevTreeError {
    /// The magic number FDT_MAGIC was not found at the start of the
    /// structure.
    InvalidMagicNumber,

    /// Unable to safely read data from the given device tree using the supplied offset
    InvalidOffset,

    /// The data was not formatted as expected.  This likely indicates an error in the Device Tree
    /// we're parsing.
    ParseError,

    /// While trying to convert a string that was supposed to be ASCII, invalid
    /// `Str` sequences were encounter.
    ///
    /// Note, the underlying type will differ based on use of the `ascii` feature.
    StrError(StrError),

    /// The device tree version is not supported by this library.
    VersionNotSupported,
    Eof,
}

impl From<SliceReadError> for DevTreeError {
    fn from(_: SliceReadError) -> DevTreeError {
        DevTreeError::ParseError
    }
}

impl From<StrError> for DevTreeError {
    fn from(e: StrError) -> DevTreeError {
        DevTreeError::StrError(e)
    }
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
    unsafe fn ptr_at<T>(&self, offset: usize) -> Result<*const T, DevTreeError> {
        if offset + size_of::<T>() > self.buf.len() {
            Err(DevTreeError::InvalidOffset)
        } else {
            Ok(self.buf.as_ptr().add(offset) as *const T)
        }
    }

    /// Returns an iterator over the Dev Tree "5.3 Memory Reservation Blocks"
    #[inline]
    #[must_use]
    pub fn reserved_entries(&self) -> iters::DevTreeReserveEntryIter {
        iters::DevTreeReserveEntryIter::new(self)
    }

    /// Returns an iterator over [`DevTreeNode`] objects
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> iters::DevTreeNodeIter {
        iters::DevTreeNodeIter::new(self)
    }

    /// Returns an iterator over objects within the [`DevTreeItem`] enum
    #[inline]
    #[must_use]
    pub fn items(&self) -> iters::DevTreeIter {
        iters::DevTreeIter::new(self)
    }

    /// Returns the root [`DevTreeNode`] object of the device tree (if it exists).
    #[inline]
    pub fn root(&self) -> Option<DevTreeNode> {
        self.nodes().next()
    }

    /// Map the supplied predicate over the [`DevTreeItem`] enum.
    ///
    /// If the predicate returns `true`, Some(([`DevTreeItem`], [`iters::DevTreeIter`])) will be returned.
    /// The [`iters::DevTreeIter`] may be used to continue searching through the tree.
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
    ///     while let Some((DevTreeItem::Prop(compatible_prop), mut iter)) = iter.find(is_uart_compatible) {
    ///         # assert!(false, "Found uart node that should not have existed.");
    ///         println!("{}", compatible_prop.parent().name()?);
    ///     }
    /// }
    /// # Ok::<(), fdt_rs::DevTreeError>(())
    /// ```
    ///
    #[inline]
    pub fn find<F>(&'a self, predicate: F) -> Option<(DevTreeItem<'a>, iters::DevTreeIter<'a>)>
    where
        F: Fn(&DevTreeItem) -> Result<bool, DevTreeError>,
    {
        iters::DevTreeIter::new(self).find(predicate)
    }

    /// Map the supplied predicate over all [`DevTreeProp`] objects
    ///
    /// If the predicate returns `true`, Some(([`DevTreeProp`], [`iters::DevTreePropIter`])) will be returned.
    /// The [`iters::DevTreePropIter`] may be used to continue searching through the tree.
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
    pub fn find_prop<F>(
        &'a self,
        predicate: F,
    ) -> Option<(DevTreeProp<'a>, iters::DevTreePropIter<'a>)>
    where
        F: Fn(&DevTreeProp) -> Result<bool, DevTreeError>,
    {
        iters::DevTreePropIter::new(self).find(predicate)
    }

    /// Map the supplied predicate over all [`DevTreeNode`] objects
    ///
    /// If the predicate returns `true`, Some(([`DevTreeItem`], [`iters::DevTreeNodeIter`])) will be returned.
    /// The [`iters::DevTreeNodeIter`] may be used to continue searching through the tree.
    #[inline]
    pub fn find_node<F>(
        &'a self,
        predicate: F,
    ) -> Option<(DevTreeNode<'a>, iters::DevTreeNodeIter<'a>)>
    where
        F: Fn(&DevTreeNode) -> Result<bool, DevTreeError>,
    {
        iters::DevTreeNodeIter::new(self).find(predicate)
    }

    /// Returns the first [`DevTreeNode`] object with the provided compatible device tree property
    /// or `None` if none exists.
    #[inline]
    pub fn find_first_compatible_node(&'a self, string: &Str) -> Option<DevTreeNode<'a>> {
        self.items().find_next_compatible_node(string)
    }
}

/// An enum which contains either a [`DevTreeNode`] or a [`DevTreeProp`]
#[derive(Clone)]
pub enum DevTreeItem<'a> {
    Node(DevTreeNode<'a>),
    Prop(DevTreeProp<'a>),
}

/// A handle to a Device Tree Node within the device tree.
#[derive(Clone)]
pub struct DevTreeNode<'a> {
    name: Result<&'a Str, DevTreeError>,
    parse_iter: iters::DevTreeIter<'a>,
}

impl<'a> DevTreeNode<'a> {
    /// Returns the name of the `DevTreeNode` (including unit address tag)
    #[inline]
    pub fn name(&'a self) -> Result<&'a Str, DevTreeError> {
        self.name
    }

    /// Returns an iterator over this node's children [`DevTreeProp`]
    #[inline]
    #[must_use]
    pub fn props(&'a self) -> iters::DevTreeNodePropIter<'a> {
        iters::DevTreeNodePropIter::new(self)
    }

    /// Returns the next [`DevTreeNode`] object with the provided compatible device tree property
    /// or `None` if none exists.
    ///
    /// # Example
    ///
    /// The following example iterates through all nodes with compatible value "virtio,mmio"
    /// and prints each node's name. (Slight modification of this example is required if using
    /// the "ascii" feature.)
    ///
    /// ```
    /// # #[cfg(not(feature = "ascii"))]
    /// # {
    /// # let mut devtree = fdt_rs::doctest::get_devtree();
    /// let compat = "virtio,mmio";
    /// # let mut count = 0;
    /// if let Some(mut cur) = devtree.root() {
    ///     while let Some(node) = cur.find_next_compatible_node(compat) {
    ///         println!("{}", node.name()?);
    ///         # count += 1;
    ///         # assert!(node.name()?.starts_with("virtio_mmio@1000"));
    ///         cur = node;
    ///     }
    /// }
    /// # assert!(count == 8);
    /// # }
    /// # Ok::<(), fdt_rs::DevTreeError>(())
    /// ```
    #[inline]
    pub fn find_next_compatible_node(&self, string: &crate::Str) -> Option<DevTreeNode<'a>> {
        self.parse_iter.find_next_compatible_node(string)
    }
}

/// A handle to a [`DevTreeNode`]'s Device Tree Property
#[derive(Clone)]
pub struct DevTreeProp<'a> {
    parent_iter: iters::DevTreeIter<'a>,
    state: DevTreePropState<'a>,
}

impl<'a> DevTreeProp<'a> {
    #[inline]
    fn fdt(&'a self) -> &'a DevTree<'a> {
        self.parent_iter.fdt
    }

    /// Returns the name of the property within the device tree.
    #[inline]
    pub fn name(&'a self) -> Result<&'a Str, DevTreeError> {
        self.state.name(self.fdt())
    }

    /// Returns the node which this property is attached to
    #[inline]
    #[must_use]
    pub fn parent(&self) -> DevTreeNode<'a> {
        self.parent_iter.clone().next_node().unwrap()
    }

    /// Returns the length of the property value within the device tree
    #[inline]
    #[must_use]
    pub fn length(&self) -> usize {
        self.state.length()
    }

    /// Read a big-endian [`u32`] from the provided offset in this device tree property's value.
    /// Convert the read value into the machines' native [`u32`] format and return it.
    ///
    /// If an offset which would cause this read to access memory outside of this property's value
    /// an [`Err`] containing [`DevTreeError::InvalidOffset`] will be returned.
    ///
    /// # Safety
    ///
    /// Device Tree Properties are not strongly typed therefore any dereference could return
    /// unexpected data.
    ///
    /// This method will access memory using [`core::ptr::read_unaligned`], therefore an unaligned
    /// offset may be provided.
    ///
    /// This method will *not* panic.
    #[inline]
    pub unsafe fn get_u32(&self, offset: usize) -> Result<u32, DevTreeError> {
        self.state.get_u32(offset)
    }

    /// Read a big-endian [`u64`] from the provided offset in this device tree property's value.
    /// Convert the read value into the machines' native [`u64`] format and return it.
    ///
    /// If an offset which would cause this read to access memory outside of this property's value
    /// an [`Err`] containing [`DevTreeError::InvalidOffset`] will be returned.
    ///
    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_u64(&self, offset: usize) -> Result<u64, DevTreeError> {
        self.state.get_u64(offset)
    }

    /// A Phandle is simply defined as a u32 value, as such this method performs the same action as
    /// [`self.get_u32`]
    ///
    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_phandle(&self, offset: usize) -> Result<Phandle, DevTreeError> {
        self.state.get_phandle(offset)
    }

    /// Returns the string property as a string if it can be parsed as one.
    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_str(&'a self) -> Result<&'a Str, DevTreeError> {
        self.state.get_str()
    }

    /// Returns the string at the given offset within the property.
    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_str_at(&'a self, offset: usize) -> Result<&'a Str, DevTreeError> {
        self.state.get_str_at(offset)
    }

    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_str_count(&self) -> Result<usize, DevTreeError> {
        self.state.get_str_count()
    }

    /// Fills the supplied slice of references with [`Str`] slices parsed from the given property.
    /// If parsing is successful, the number of parsed strings will be returned.
    ///
    /// If an error occurred parsing one or more of the strings (E.g. they were not valid
    /// UTF-8/ASCII strings) an [`Err`] of type [`DevTreeError`] will be returned.
    /// ```
    /// # #[cfg(not(feature = "ascii"))]
    /// # {
    /// # use fdt_rs::Str;
    /// # let mut devtree = fdt_rs::doctest::get_devtree();
    /// # let node = devtree.nodes().next().unwrap();
    /// # let prop = node.props().next().unwrap();
    /// # unsafe {
    /// // Get the number of possible strings
    /// if let Ok(count) = prop.get_str_count() {
    ///
    ///     // Allocate a vector to store the strings
    ///     let mut vec: Vec<Option<&Str>> = vec![None; count];
    ///
    ///     // Read and parse the strings
    ///     if let Ok(_) = prop.get_strlist(&mut vec) {
    ///         let mut iter = vec.iter();
    ///
    ///         // Print out all the strings we found in the property
    ///         while let Some(Some(s)) = iter.next() {
    ///             print!("{} ", s);
    ///         }
    ///     }
    /// }
    /// # }
    /// # }
    /// # Ok::<(), fdt_rs::DevTreeError>(())
    /// ```
    ///
    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_strlist(
        &'a self,
        list: &mut [Option<&'a Str>],
    ) -> Result<usize, DevTreeError> {
        self.state.get_strlist(list)
    }

    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_raw(&self) -> &'a [u8] {
        self.state.get_raw()
    }
}

// When the doctest feature is enabled, add these utility functions.
#[cfg(feature = "doctest")]
pub mod doctest {
    use crate::*;

    // Include the readme for doctests
    // https://doc.rust-lang.org/rustdoc/documentation-tests.html#include-items-only-when-collecting-doctests
    //
    // Ignore ascii since we don't want to have to bother with string conversion.
    #[cfg(RUSTC_IS_NIGHTLY)]
    #[cfg(not(feature = "ascii"))]
    #[doc(include = "../README.md")]
    pub struct ReadmeDoctests;

    #[repr(align(4))]
    struct _Wrapper<T>(T);
    pub const FDT: &[u8] = &_Wrapper(*include_bytes!("../tests/riscv64-virt.dtb")).0;

    pub fn get_devtree() -> DevTree<'static> {
        unsafe { DevTree::new(FDT).unwrap() }
    }
}

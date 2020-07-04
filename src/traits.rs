use core::str::from_utf8;

use crate::base::DevTree;
use crate::error::DevTreeError;
use crate::prelude::*;
use crate::spec::Phandle;

#[cfg(doc)]
use crate::base::DevTreeProp;

#[doc(hidden)]
pub trait DevTreePropStateBase<'r, 'dt: 'r> {
    fn propbuf(&'r self) -> &'dt [u8];
    fn nameoff(&'r self) -> usize;
    fn fdt(&'r self) -> &'r DevTree<'dt>;
}

pub trait IterableDevTree<'a, 'dt:'a> {
    type TreeNode;
    type TreeIter;
    type NodeIter;
    type PropIter;

    #[must_use]
    fn props(&'a self) -> Self::PropIter;
    #[must_use]
    fn nodes(&'a self) -> Self::NodeIter;
    #[must_use]
    fn items(&'a self) -> Self::TreeIter;
    fn find_first_compatible_node(&'a self, string: &str) -> Option<Self::TreeNode>;
    #[must_use]
    fn buf(&'a self) -> &'dt [u8];
    fn root(&'a self) -> Option<Self::TreeNode>;
}

pub trait DevTreePropState<'r, 'dt: 'r>: DevTreePropStateBase<'r, 'dt> {
    /// Returns the name of the property within the device tree.
    #[inline]
    fn name(&'r self) -> Result<&'r str, DevTreeError> {
        PropTraitWrap(self).get_prop_str()
    }

    /// Returns the length of the property value within the device tree
    #[inline]
    #[must_use]
    fn length(&'r self) -> usize {
        self.propbuf().len()
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
    unsafe fn get_u32(&'r self, offset: usize) -> Result<u32, DevTreeError> {
        self.propbuf()
            .read_be_u32(offset)
            .or(Err(DevTreeError::InvalidOffset))
    }

    /// Read a big-endian [`u64`] from the provided offset in this device tree property's value.
    /// Convert the read value into the machines' native [`u64`] format and return it.
    ///
    /// If an offset which would cause this read to access memory outside of this property's value
    /// an [`Err`] containing [`DevTreeError::InvalidOffset`] will be returned.
    ///
    /// # Safety
    ///
    /// See the safety note of [`DevTreePropState::get_u32`]
    #[inline]
    unsafe fn get_u64(&'r self, offset: usize) -> Result<u64, DevTreeError> {
        self.propbuf()
            .read_be_u64(offset)
            .or(Err(DevTreeError::InvalidOffset))
    }

    /// A Phandle is simply defined as a u32 value, as such this method performs the same action as
    /// [`self.get_u32`]
    ///
    /// # Safety
    ///
    /// See the safety note of [`DevTreePropState::get_u32`]
    #[inline]
    unsafe fn get_phandle(&'r self, offset: usize) -> Result<Phandle, DevTreeError> {
        self.propbuf()
            .read_be_u32(offset)
            .or(Err(DevTreeError::InvalidOffset))
    }

    /// Returns the string property as a string if it can be parsed as one.
    /// # Safety
    ///
    /// See the safety note of [`DevTreePropState::get_u32`]
    #[inline]
    unsafe fn get_str(&'dt self) -> Result<&'dt str, DevTreeError> {
        self.get_str_at(0)
    }

    /// Returns the string at the given offset within the property.
    /// # Safety
    ///
    /// See the safety note of [`DevTreePropState::get_u32`]
    #[inline]
    unsafe fn get_str_at(&'dt self, offset: usize) -> Result<&'dt str, DevTreeError> {
        match PropTraitWrap(self).get_string(offset, true) {
            // Note, unwrap invariant is safe.
            // get_string returns Some(s) when second opt is true
            Ok((_, s)) => Ok(s.unwrap()),
            Err(e) => Err(e),
        }
    }

    /// # Safety
    ///
    /// See the safety note of [`DevTreePropState::get_u32`]
    #[inline]
    unsafe fn get_str_count(&'dt self) -> Result<usize, DevTreeError> {
        PropTraitWrap(self).iter_str_list(None)
    }

    /// Fills the supplied slice of references with [`str`] slices parsed from the given property.
    /// If parsing is successful, the number of parsed strings will be returned.
    ///
    /// If an error occurred parsing one or more of the strings (E.g. they were not valid
    /// UTF-8/ASCII strings) an [`Err`] of type [`DevTreeError`] will be returned.
    /// ```
    /// # #[cfg(not(feature = "ascii"))]
    /// # {
    /// # let mut devtree = fdt_rs::doctest::get_devtree();
    /// # let node = devtree.nodes().next().unwrap();
    /// # let prop = node.props().next().unwrap();
    /// # unsafe {
    /// // Get the number of possible strings
    /// if let Ok(count) = prop.get_str_count() {
    ///
    ///     // Allocate a vector to store the strings
    ///     let mut vec: Vec<Option<&str>> = vec![None; count];
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
    /// See the safety note of [`DevTreePropState::get_u32`]
    #[inline]
    unsafe fn get_strlist(&'dt self, list: &mut [Option<&'dt str>]) -> Result<usize, DevTreeError> {
        PropTraitWrap(self).iter_str_list(Some(list))
    }

    /// # Safety
    ///
    /// See the safety note of [`DevTreePropState::get_u32`]
    #[inline]
    unsafe fn get_raw(&'r self) -> &'r [u8] {
        self.propbuf()
    }
}

struct PropTraitWrap<'r, T: ?Sized>(&'r T);

impl<'r, 'dt: 'r, T: DevTreePropState<'r, 'dt> + ?Sized> PropTraitWrap<'r, T> {
    fn get_prop_str(&self) -> Result<&'r str, DevTreeError> {
        unsafe {
            let str_offset = self.0.fdt().off_dt_strings() + self.0.nameoff();
            let name = self.0.fdt().buf().read_bstring0(str_offset)?;
            Ok(from_utf8(name)?)
        }
    }

    /// # Safety
    ///
    /// See the safety note of [`DevTreePropState::get_u32`]
    unsafe fn get_string(
        &self,
        offset: usize,
        parse: bool,
    ) -> Result<(usize, Option<&'dt str>), DevTreeError> {
        match self.0.propbuf().read_bstring0(offset) {
            Ok(res_u8) => {
                // Include null byte
                let len = res_u8.len() + 1;

                if parse {
                    match from_utf8(res_u8) {
                        Ok(s) => Ok((len, Some(s))),
                        Err(e) => Err(e.into()),
                    }
                } else {
                    Ok((len, None))
                }
            }
            Err(e) => Err(e.into()),
        }
    }

    /// # Safety
    ///
    /// See the safety note of [`DevTreePropState::get_u32`]
    unsafe fn iter_str_list(
        &self,
        mut list_opt: Option<&mut [Option<&'dt str>]>,
    ) -> Result<usize, DevTreeError> {
        let mut offset = 0;
        for count in 0.. {
            if offset == self.0.length() {
                return Ok(count);
            }

            let (len, s) = self.get_string(offset, list_opt.is_some())?;
            offset += len;

            if let Some(list) = list_opt.as_deref_mut() {
                // Note, unwrap invariant is safe.
                // get_string returns Some(s) if we ask it to parse and it returns Ok
                (*list)[count] = Some(s.unwrap());
            };
        }
        // For some reason infinite for loops need unreachable.
        unreachable!();
    }
}

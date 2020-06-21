use crate::*;
use crate::iters::AssociatedOffset;


#[derive(Clone)]
pub(crate) struct DevTreePropState<'dt> {
    pub propbuf: &'dt [u8],
    pub nameoff: AssociatedOffset<'dt>,
}

pub(crate) trait DevTreePropStateTrait<'dt> {
    fn propbuf() -> &'dt [u8];
    fn nameoff() -> AssociatedOffset<'dt>;
    fn fdt() -> &'dt DevTree<'dt>;
}

impl<'dt> DevTreePropState<'dt> {
    /// Returns the name of the property within the device tree.
    #[inline]
    pub fn name(&self, fdt: &'dt DevTree<'dt>) -> Result<&'dt Str, DevTreeError> {
        self.get_prop_str(fdt)
    }

    /// Returns the length of the property value within the device tree
    #[inline]
    #[must_use]
    pub fn length(&self) -> usize {
        self.propbuf.len()
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
        self.propbuf
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
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_u64(&self, offset: usize) -> Result<u64, DevTreeError> {
        self.propbuf
            .read_be_u64(offset)
            .or(Err(DevTreeError::InvalidOffset))
    }

    /// A Phandle is simply defined as a u32 value, as such this method performs the same action as
    /// [`self.get_u32`]
    ///
    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_phandle(&self, offset: usize) -> Result<Phandle, DevTreeError> {
        self.propbuf
            .read_be_u32(offset)
            .or(Err(DevTreeError::InvalidOffset))
    }

    /// Returns the string property as a string if it can be parsed as one.
    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_str(&'dt self) -> Result<&'dt Str, DevTreeError> {
        self.get_str_at(0)
    }

    /// Returns the string at the given offset within the property.
    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_str_at(&'dt self, offset: usize) -> Result<&'dt Str, DevTreeError> {
        match self.get_string(offset, true) {
            // Note, unwrap invariant is safe.
            // get_string returns Some(s) when second opt is true
            Ok((_, s)) => Ok(s.unwrap()),
            Err(e) => Err(e),
        }
    }

    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_str_count(&self) -> Result<usize, DevTreeError> {
        self.iter_str_list(None)
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
        &'dt self,
        list: &mut [Option<&'dt Str>],
    ) -> Result<usize, DevTreeError> {
        self.iter_str_list(Some(list))
    }

    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    #[inline]
    pub unsafe fn get_raw(&self) -> &'dt [u8] {
        self.propbuf
    }

    fn get_prop_str(&self, fdt: &DevTree<'dt>) -> Result<&'dt Str, DevTreeError> {
        unsafe {
            let str_offset = fdt.off_dt_strings() + self.nameoff.0;
            let name = fdt.buf.read_bstring0(str_offset)?;
            Ok(bytes_as_str(name)?)
        }
    }

    /// # Safety
    ///
    /// See the safety note of [`DevTreeProp::get_u32`]
    unsafe fn get_string(
        &'dt self,
        offset: usize,
        parse: bool,
    ) -> Result<(usize, Option<&'dt Str>), DevTreeError> {
        match self.propbuf.read_bstring0(offset) {
            Ok(res_u8) => {
                // Include null byte
                let len = res_u8.len() + 1;

                if parse {
                    match bytes_as_str(res_u8) {
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
    /// See the safety note of [`DevTreeProp::get_u32`]
    unsafe fn iter_str_list(
        &'dt self,
        mut list_opt: Option<&mut [Option<&'dt Str>]>,
    ) -> Result<usize, DevTreeError> {
        let mut offset = 0;
        for count in 0.. {
            if offset == self.length() {
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

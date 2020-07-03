use crate::priv_util::SliceReadError;
use core::str::Utf8Error;

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
    /// `str` sequences were encounter.
    StrError(Utf8Error),

    /// The device tree version is not supported by this library.
    VersionNotSupported,

    /// There wasn't enough memory to create a [`DevTreeIndex`].
    NotEnoughMemory,
    Eof,
}

impl From<SliceReadError> for DevTreeError {
    fn from(_: SliceReadError) -> DevTreeError {
        DevTreeError::ParseError
    }
}

impl From<Utf8Error> for DevTreeError {
    fn from(e: Utf8Error) -> DevTreeError {
        DevTreeError::StrError(e)
    }
}

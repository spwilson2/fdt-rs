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
//! ```
#![deny(clippy::all, clippy::cargo)]
#![allow(clippy::as_conversions)]
// Test the readme if using nightly.
#![cfg_attr(RUSTC_IS_NIGHTLY, feature(external_doc))]
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(any(feature = "std", feature = "alloc"))]
extern crate alloc;
#[cfg(feature = "std")]
extern crate core;
extern crate endian_type_rs as endian_type;
#[macro_use]
extern crate memoffset;
#[macro_use]
extern crate static_assertions;
extern crate unsafe_unwrap;

pub mod error;

mod priv_util;

pub mod base;
pub mod index;
pub mod prelude;
pub mod spec;
pub mod traits;

// When the doctest feature is enabled, add these utility functions.
#[cfg(feature = "doctest")]
pub mod doctest {
    use crate::*;

    // Include the readme for doctests
    // https://doc.rust-lang.org/rustdoc/documentation-tests.html#include-items-only-when-collecting-doctests
    #[cfg(RUSTC_IS_NIGHTLY)]
    #[doc(include = "../README.md")]
    pub struct ReadmeDoctests;

    #[repr(align(4))]
    struct _Wrapper<T>(T);
    pub const FDT: &[u8] = &_Wrapper(*include_bytes!("../tests/riscv64-virt.dtb")).0;

    pub fn get_devtree() -> DevTree<'static> {
        unsafe { DevTree::new(FDT).unwrap() }
    }
}

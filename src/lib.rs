//! A flattened device tree (FDT) parser for embedded, low memory, and safety-critical no-std
//! environments.
//!
//! Includes the following features:
//!
//! * [Low-level FDT parsing utilities to build your own parser](base::parse)
//! * [Simple utilites based on in-order parsing of the FDT](base)
//! * [Performant utilities built on a no-alloc index](index)
//!
//! ## Features
//!
//! This crate can be used without the standard library (`#![no_std]`) by disabling
//! the default `std` feature. To use `no-std` place the following in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies.fdt-rs]
//! version = "x"
//! default-features = false
//! ```
//!
//! ## Examples
//!
//!
#![deny(clippy::all, clippy::cargo)]
#![allow(clippy::as_conversions)]
// Test the readme if using nightly.
#![cfg_attr(RUSTC_IS_NIGHTLY, feature(external_doc))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate core;
extern crate endian_type_rs as endian_type;
#[macro_use]
extern crate memoffset;
#[macro_use]
extern crate static_assertions;
extern crate unsafe_unwrap;

pub mod error;
pub mod base;
pub mod index;
pub mod prelude;
pub mod spec;

#[doc(hidden)]
pub mod common;

pub(crate) mod priv_util;

// When the doctest feature is enabled, add these utility functions.
#[cfg(feature = "doctest")]
pub mod doctest {
    // Include the readme for doctests
    // https://doc.rust-lang.org/rustdoc/documentation-tests.html#include-items-only-when-collecting-doctests
    #[cfg(RUSTC_IS_NIGHTLY)]
    #[doc(include = "../README.md")]
    pub struct ReadmeDoctests;

    #[repr(align(4))]
    struct _Wrapper<T>(T);
    pub const FDT: &[u8] = &_Wrapper(*include_bytes!("../tests/riscv64-virt.dtb")).0;
}

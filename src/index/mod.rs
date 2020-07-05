//! Performant device tree parsing utils provided using an index.
//!
//! This module provides advanced Device Tree utilities. These utilites depend on an index built
//! over a parsed device tree. This index may be built without an allocator. In order to build the
//! index, only a single `[u8]` buffer is required.
//!
//! # Examples
//!
//! ## Initialization
//!
//! ```
//! # use fdt_rs::doctest::FDT;
//! use fdt_rs::prelude::*;
//! use fdt_rs::base::*;
//! use fdt_rs::index::*;
//!
//! // Get access to a flattened device tree buffer.
//! let fdt: &[u8] = FDT;
//!
//! // Create the device tree parser
//! let devtree = unsafe { DevTree::new(fdt) }
//!     .expect("Buffer does not contain a device tree.");
//!
//! // Get the layout required to build an index
//! let layout = DevTreeIndex::get_layout(&devtree)
//!     .expect("Failed to parse DTB - it is invalid.");
//!
//! // Allocate memory for the index.
//! //
//! // This could be performed without a dynamic allocation
//! // if we allocated a static buffer or want to provide a
//! // raw buffer into uninitialized memory.
//! let mut vec = vec![0u8; layout.size() + layout.align()];
//! let raw_slice = vec.as_mut_slice();
//!
//! // Create the index of the device tree.
//! let index = DevTreeIndex::new(devtree, raw_slice).unwrap();
//!
//! ```
//! ## Search
//!
//! ```
//! # use fdt_rs::doctest::*;
//! # let (index, _) = doctest_index();
//! // Find a DevTreeIndexNode which has "compatible" = "ns16550a"
//! let node = index.find_first_compatible_node("ns16550a")
//!     .expect("No node found!");
//! ```
//! ## Iterative Search
//! ```
//! # use fdt_rs::doctest::*;
//! # let (index, _) = doctest_index();
//!
//! let mut tree_iter = index.items();
//!
//! while let Some(node) = tree_iter.next_compatible_node("virtio,mmio") {
//! }
//! ```
//!

#[doc(hidden)]
pub mod item;
#[doc(hidden)]
pub mod node;
#[doc(hidden)]
pub mod prop;
#[doc(hidden)]
pub mod tree;

pub mod iters;

#[doc(inline)]
pub use item::DevTreeIndexItem;
#[doc(inline)]
pub use node::DevTreeIndexNode;
#[doc(inline)]
pub use prop::DevTreeIndexProp;
#[doc(inline)]
pub use tree::DevTreeIndex;

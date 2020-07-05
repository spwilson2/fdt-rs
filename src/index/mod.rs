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
//! # use fdt_rs::base::*;
//! # use fdt_rs::index::*;
//! # use fdt_rs::doctest::FDT;

//! // Create the device tree parser
//! let devtree = unsafe{ DevTree::new(FDT) }.unwrap();
//!
//! // Get the layout required to build an index
//! let layout = DevTreeIndex::get_layout(&devtree).unwrap();
//!
//! // Allocate memory for the index.  
//! // 
//! // This could be performed without a dynamic allocation
//! // if we allocated a static buffer or want to provide a
//! // raw buffer into uninitialized memory.
//! let mut vec = vec![0u8; layout.size() + layout.align()];
//! let idx = DevTreeIndex::new(devtree, vec.as_mut_slice()).unwrap();
//!
//! ```
//! ## Search
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

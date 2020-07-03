//! This module is only enabled for `#[cfg(any(feature = "alloc", feature = "std"))]`.
//!
//! This module provides advanced DevTree utilities which require an index over a flattened device
//! tree to remain performant. As such, we rely on an allocator to provide heap allocations to
//! generate and store this index.
//!
//! (Ideally we wouldn't require a full system allocator and could just use a small memory pool
//! allocator. If anyone knows of such an allocator/interface reach out and we might add this.)

#![allow(dead_code)] // TODO/FIXME
#![allow(unused_variables)]
#![allow(unused_imports)]

pub mod item;
pub mod iter;
pub mod node;
pub mod prop;
pub mod tree;

pub use item::DevTreeIndexItem;
pub use node::DevTreeIndexNode;
pub use prop::DevTreeIndexProp;
pub use tree::DevTreeIndex;

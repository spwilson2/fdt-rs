//! This module is only enabled for `#[cfg(any(feature = "alloc", feature = "std"))]`.
//!
//! This module provides advanced DevTree utilities which require an index over a flattened device
//! tree to remain performant. As such, we rely on an allocator to provide heap allocations to
//! generate and store this index.
//!
//! (Ideally we wouldn't require a full system allocator and could just use a small memory pool
//! allocator. If anyone knows of such an allocator/interface reach out and we might add this.)

#[doc(hidden)]
pub mod item;
pub mod iter;
#[doc(hidden)]
pub mod node;
#[doc(hidden)]
pub mod prop;
#[doc(hidden)]
pub mod tree;

#[doc(inline)]
pub use item::DevTreeIndexItem;
#[doc(inline)]
pub use node::DevTreeIndexNode;
#[doc(inline)]
pub use prop::DevTreeIndexProp;
#[doc(inline)]
pub use tree::DevTreeIndex;

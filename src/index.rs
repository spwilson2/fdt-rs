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
use crate::iters::{AssociatedOffset, FindNext};
use crate::unsafe_unwrap::UnsafeUnwrap;
use crate::*;

use core::alloc::Layout;
use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ptr::{null, null_mut};
use core::str;

unsafe fn ptr_in<T>(buf: &[u8], ptr: *const T) -> bool {
    // Make sure we dont' go over the buffer
    let mut res = buf.as_ptr() as usize + buf.len() > (ptr as usize + size_of::<T>());
    // Make sure we don't go under the buffer
    res &= buf.as_ptr() as usize <= ptr as usize;
    return res;
}

unsafe fn aligned_ptr_in<T>(buf: &[u8], offset: usize) -> Result<*mut T, DevTreeError> {
    let ptr = buf.as_ptr().add(offset);

    let ptr = ptr.add(ptr.align_offset(align_of::<T>())) as *mut T;
    if !ptr_in(buf, ptr) {
        return Err(DevTreeError::NotEnoughMemory);
    }
    Ok(ptr)
}

pub struct DTIProp<'dt> {
    propbuf: &'dt [u8],
    nameoff: AssociatedOffset<'dt>,
}

#[derive(Debug)]
pub struct DevTreeIndex<'i, 'dt: 'i> {
    fdt: DevTree<'dt>,
    root: *const DTINode<'i, 'dt>,
}

struct DTIBuilder<'i, 'dt: 'i> {
    buf: &'i mut [u8],
    cur_node: *mut DTINode<'i, 'dt>,
    prev_new_node: *mut DTINode<'i, 'dt>,
    front_off: usize,

    /// Devtree Props may only occur before child nodes.
    /// We'll call this the "node_header".
    in_node_header: bool,
}

struct DTINode<'i, 'dt: 'i> {
    parent: *const Self,
    first_child: *const Self,
    /// `next` is either
    /// 1. the next sibling node
    /// 2. the next node in DFS (some higher up node)
    /// It is 1 if (*next).parent == self.parent, otherwise it is 2.
    next: *const Self,
    name: &'dt [u8],

    //NOTE: We store props like C arrays.
    // This the number of props after this node in memory.
    // Props are a packed array after each node.
    num_props: usize,
    _index: PhantomData<&'i u8>,
}

impl<'i, 'dt: 'i> DTINode<'i, 'dt> {
    unsafe fn prop_unchecked(&self, idx: usize) -> &'i DTIProp<'dt> {
        // Get the pointer to the props after ourself.
        let prop_ptr = (self as *const Self).add(1) as *const DTIProp;
        return &*prop_ptr.add(idx);
    }
}

impl<'i, 'dt: 'i> DTIBuilder<'i, 'dt> {
    fn allocate_aligned_ptr<T>(&mut self) -> Result<*mut T, DevTreeError> {
        unsafe {
            let ptr = aligned_ptr_in::<T>(self.buf, self.front_off)?;
            self.front_off = ptr.add(1) as usize - self.buf.as_ptr() as usize;
            Ok(ptr)
        }
    }

    pub fn parsed_node(&mut self, node: &iters::ParsedBeginNode<'dt>) -> Result<(), DevTreeError> {
        unsafe {
            self.in_node_header = true;

            let new_ptr = self.allocate_aligned_ptr::<DTINode>()?;
            let parent = self.cur_node;

            // Write the data
            *new_ptr = DTINode {
                parent,

                // set by the next node we create
                first_child: null_mut(),
                // set by the next node we create
                next: null_mut(),

                name: node.name,
                num_props: 0,
                _index: PhantomData,
            };

            if !parent.is_null() {
                debug_assert!(
                    self.prev_new_node != null_mut(),
                    "cur_node should not have been initialized without also intializing \
                    prev_new_node"
                );

                (*self.prev_new_node).next = new_ptr;
                if !(*parent).next.is_null() {
                    let prev_sibling = (*parent).next as *mut DTINode;
                    (*prev_sibling).next = new_ptr;
                }
                (*parent).next = new_ptr;

                // If this new node is the first node that follows the current one, it is the current's
                // first child.
                if (*parent).first_child.is_null() {
                    (*parent).first_child = new_ptr;
                }
            }

            // Save the new node ptr.
            self.cur_node = new_ptr;
            self.prev_new_node = new_ptr;
        }

        Ok(())
    }

    pub fn parsed_prop(&mut self, prop: &iters::ParsedProp<'dt>) -> Result<(), DevTreeError> {
        if !self.in_node_header {
            return Err(DevTreeError::ParseError);
        }

        unsafe {
            let new_ptr = self.allocate_aligned_ptr::<DTIProp>()?;
            (*self.cur_node).num_props += 1;
            *new_ptr = DTIProp::from(prop);
        }

        Ok(())
    }

    pub fn parsed_end_node(&mut self) -> Result<(), DevTreeError> {
        // There were more EndNode tokens than BeginNode ones.
        if self.cur_node.is_null() {
            return Err(DevTreeError::ParseError);
        }
        // Unsafe is Ok.
        // Lifetime : self.cur_node is a pointer into a buffer with the same lifetime as self
        // Alignment: parsed_node verifies alignment when creating self.cur_node
        // NonNull  : We check that self.cur_node is non-null above
        // Mutability: We cast from a *const to a *mut.
        //             We're the only thread which has access to the buffer at this time, so this
        //             is thread-safe.
        unsafe {
            // Change the current node back to the parent.
            self.cur_node = (*self.cur_node).parent as *mut DTINode;
        }

        // We are no longer in a node header.
        // We are either going to see a new node next or parse another end_node.
        self.in_node_header = false;

        Ok(())
    }
}

impl<'i, 'dt: 'i> DevTreeIndex<'i, 'dt> {
    // Note: Our parsing method is unsafe - particularly due to its use of pointer arithmetic.
    //
    // We decide this is worth it for the following reasons:
    // - It requires no allocator.
    // - It has incredibly low overhead.
    //   - This parsing method only requires a single allocation. (The buffer given as buf)
    //   - This parsing method only requires a single iteration over the FDT.
    // - It is very easy to test in isolation; parsing is entirely enclosed to this module.
    unsafe fn init_builder<'a>(
        buf: &'i mut [u8],
        iter: &mut iters::DevTreeParseIter<'a, 'dt>,
    ) -> Result<DTIBuilder<'i, 'dt>, DevTreeError> {
        let mut builder = DTIBuilder {
            front_off: 0,
            buf,
            cur_node: null_mut(),
            prev_new_node: null_mut(),
            in_node_header: false,
        };

        for tok in iter {
            match tok {
                iters::ParsedTok::BeginNode(node) => {
                    builder.parsed_node(&node)?;
                    return Ok(builder);
                }
                iters::ParsedTok::Nop => continue,
                _ => return Err(DevTreeError::ParseError),
            }
        }

        Err(DevTreeError::ParseError)
    }

    pub fn get_layout(fdt: &'i DevTree<'dt>) -> Result<Layout, DevTreeError> {
        // Size may require alignment of DTINode.
        let mut size = 0usize;

        // We assert this because it makes size calculations easier.
        // We don't have to worry about re-aligning between props and nodes.
        // If they didn't have the same alignment, we would have to keep track
        // of the last node and re-align depending on the last seen type.
        //
        // E.g. If we saw one node, two props, and then two nodes:
        //
        // size = \
        // align_of::<DTINode> + size_of::<DTINode>
        // + align_of::<DTIProp> + size_of::<DTIProp>
        // + size_of::<DTIProp>
        // + size_of::<DTIProp>
        // + align_of::<DTINode> + size_of::<DTINode>
        // + size_of::<DTINode>
        const_assert_eq!(align_of::<DTINode>(), align_of::<DTIProp>());

        for item in iters::DevTreeIter::new(fdt) {
            match item {
                DevTreeItem::Node(_) => size += size_of::<DTINode>(),
                DevTreeItem::Prop(_) => size += size_of::<DTIProp>(),
            }
        }

        // Unsafe okay.
        // - Size is not likely to be usize::MAX. (There's no way we find that many nodes.)
        // - Align is a result of align_of, so it will be a non-zero power of two
        unsafe {
            return Ok(Layout::from_size_align_unchecked(
                size,
                align_of::<DTINode>(),
            ));
        }
    }

    pub fn new(fdt: DevTree<'dt>, buf: &'i mut [u8]) -> Result<Self, DevTreeError> {

        let mut iter = iters::DevTreeParseIter::new(&fdt);

        let mut builder = unsafe { Self::init_builder(buf, &mut iter) }?;

        let this = Self {
            fdt,
            root: builder.cur_node,
        };

        // The buffer will be split into two parts, front and back:
        //
        // Front will be used as a temporary work section to  build the nodes as we parse them.
        // The back will be used to save completely parsed nodes.
        for tok in iter {
            match tok {
                iters::ParsedTok::BeginNode(node) => {
                    builder.parsed_node(&node)?;
                }
                iters::ParsedTok::Prop(prop) => {
                    builder.parsed_prop(&prop)?;
                }
                iters::ParsedTok::EndNode => {
                    builder.parsed_end_node()?;
                }
                iters::ParsedTok::Nop => continue,
            }
        }

        Ok(this)
    }

    pub fn nodes(&self) -> DevTreeIndexNodeIter<'_, 'i, 'dt> {
        DevTreeIndexNodeIter::new(self)
    }

    #[inline]
    #[must_use]
    pub fn items(&self) -> DevTreeIndexItemIter {
        DevTreeIndexItemIter::new(self)
    }

    #[inline]
    pub fn root(&self) -> Option<DevTreeIndexNode<'_, 'i, 'dt>> {
        self.nodes().next()
    }

    #[inline]
    pub fn find_item<F>(&'_ self, predicate: F) -> Option<(DevTreeIndexItem<'_, 'i, 'dt>, DevTreeIndexItemIter<'_, 'i, 'dt>)>
    where
        F: Fn(&DevTreeIndexItem) -> Result<bool, DevTreeError>,
    {
        DevTreeIndexItemIter::new(self).find_next(predicate)
    }

    #[inline]
    pub fn find_prop<F>(
        &self,
        predicate: F,
    ) -> Option<(DevTreeIndexProp<'_, 'i, 'dt>, DevTreeIndexPropIter<'_, 'i, 'dt>)>
    where
        F: Fn(&DevTreeIndexProp) -> Result<bool, DevTreeError>,
    {
        DevTreeIndexPropIter::new(self).find_next(predicate)
    }

    #[inline]
    pub fn find_node<F>(
        &self,
        predicate: F,
    ) -> Option<(DevTreeIndexNode<'_, 'i, 'dt>, DevTreeIndexNodeIter<'_, 'i, 'dt>)>
    where
        F: Fn(&DevTreeIndexNode) -> Result<bool, DevTreeError>,
    {
        DevTreeIndexNodeIter::new(self).find_next(predicate)
    }

    #[inline]
    pub fn find_first_compatible_node(&'_ self, string: &Str) -> Option<DevTreeIndexNode<'_, 'i, 'dt>> {
        let prop = self.find_prop(move |prop| Ok(prop.name()? == "compatible" && unsafe {prop.get_str()}? == string));
        if let Some(prop) = prop {
            return Some(prop.0.node());
        }
        None
    }
}
/*
 *
 * Wrappers around the internal index types: 
 *
 * Wrappers include a reference to the index they are based on.
 */
#[derive(Clone)]
pub struct DevTreeIndexNode<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node: &'a DTINode<'i, 'dt>,
}

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexNode<'a, 'i, 'dt> {
    fn new(index: &'a DevTreeIndex<'i, 'dt>, node: &'a DTINode<'i, 'dt>) -> Self {
        Self { node, index }
    }

    pub fn name(&self) -> Result<&'dt str, DevTreeError> {
        str::from_utf8(self.node.name).map_err(|e| DevTreeError::StrError(e))
    }

    pub fn siblings(&self) -> DevTreeIndexNodeSiblingIter<'_, 'i, 'dt> {
        DevTreeIndexNodeSiblingIter::new(self)
    }

    pub fn props(&self) -> DevTreeIndexNodePropIter<'a, 'i, 'dt> {
        DevTreeIndexNodePropIter::new(self.index, self.node)
    }
}

#[derive(Clone)]
pub struct DevTreeIndexProp<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node: &'a DTINode<'i, 'dt>,
    prop: &'a DTIProp<'dt>,
}

impl<'r, 'a:'r, 'i:'a, 'dt:'i> DevTreeIndexProp<'a, 'i, 'dt> { 
    fn node(&self) -> DevTreeIndexNode<'a, 'i, 'dt> {
        DevTreeIndexNode::new(self.index, self.node)
    }
}

impl<'r, 'a:'r, 'i:'a, 'dt:'i> DevTreePropState<'r, 'dt> for DevTreeIndexProp<'a, 'i, 'dt> {}
impl<'r, 'a:'r, 'i:'a, 'dt:'i> private::DevTreePropStateBase<'r, 'dt> for DevTreeIndexProp<'a, 'i, 'dt> {
    #[inline]
    fn propbuf(&'r self) -> &'dt [u8] {
        self.prop.propbuf
    }

    #[inline]
    fn nameoff(&'r self) -> AssociatedOffset<'dt> {
        self.prop.nameoff
    }

    #[inline]
    fn fdt(&'r self) -> &'r DevTree<'dt> {
        &self.index.fdt
    }
}

impl<'dt> From<&iters::ParsedProp<'dt>> for DTIProp<'dt> {
    fn from(prop: &iters::ParsedProp<'dt>) -> Self {
        Self {
            propbuf: prop.prop_buf,
            nameoff: prop.name_offset,
        }
    }
}

#[derive(Clone)]
pub enum DevTreeIndexItem<'a, 'i: 'a, 'dt:'i> {
    Node(DevTreeIndexNode<'a, 'i, 'dt>),
    Prop(DevTreeIndexProp<'a, 'i, 'dt>),
}


/*********************************************************************************/
/************************       Iterators     ************************************/
/*********************************************************************************/


/***********************************/
/***********  DFS  *****************/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexNodeIter<'a, 'i: 'a, 'dt:'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node: Option<&'a DTINode<'i, 'dt>>,
}

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexNodeIter<'a, 'i, 'dt> {
    pub(crate) fn new(index: &'a DevTreeIndex<'i, 'dt>) -> Self {
        unsafe {
            let root_ref = index.root.as_ref().unsafe_unwrap();

            Self {
                index,
                node: Some(root_ref),
            }
        }
    }
}

impl FindNext for DevTreeIndexNodeIter<'_, '_, '_> {}
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexNodeIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexNode<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.node {
            let cur = DevTreeIndexNode {
                index: self.index,
                node,
            };

            // Unsafe is OK.
            // This is unsafe because we cast node pointers into references.
            // These references live as long as the DevTreeIndex and therefore the lifetimes match.
            unsafe {
                self.node = if let Some(next) = node.first_child.as_ref() {
                    Some(next)
                } else if let Some(next) = node.next.as_ref() {
                    Some(next)
                } else {
                    None
                }
            }
            return Some(cur);
        }
        None
    }
}

/***********************************/
/***********  Node Siblings  *******/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexNodeSiblingIter<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node: Option<&'a DTINode<'i, 'dt>>,
}

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexNodeSiblingIter<'a, 'i, 'dt> {
    pub(crate) fn new(node: &'a DevTreeIndexNode<'a, 'i, 'dt>) -> Self {
        Self {
            index: &node.index,
            node: Some(node.node),
        }
    }
}

impl FindNext for DevTreeIndexNodeSiblingIter<'_, '_, '_> {}
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexNodeSiblingIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexNode<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.node {
            let cur = DevTreeIndexNode::new(self.index, node);

            // Unsafe is OK.
            // This is unsafe because we cast node pointers into references.
            // These references live as long as the DevTreeIndex and therefore the lifetimes match.
            unsafe {
                // Set the next return value.
                self.node = if let Some(next) = node.next.as_ref() {
                    // If the next node is not a sibling (it doesn't have the same parent) then
                    // this will be our last iteration.
                    if next.parent == node.parent {
                        Some(next)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            return Some(cur);
        }
        None
    }
}

/***********************************/
/***********  Node Props ***********/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexNodePropIter<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node: &'a DTINode<'i, 'dt>,
    prop_idx: usize,
}

impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexNodePropIter<'a, 'i, 'dt> {
    fn new(index: &'a DevTreeIndex<'i, 'dt>, node: &'a DTINode<'i, 'dt>) -> Self {
        Self {
            index,
            node,
            prop_idx: 0,
        }
    }
}

impl FindNext for DevTreeIndexNodePropIter<'_, '_, '_> {}
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexNodePropIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexProp<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.prop_idx < self.node.num_props {
            // Unsafe OK, we just checked the length of props.
            let prop = unsafe { self.node.prop_unchecked(self.prop_idx) };

            self.prop_idx += 1;
            return Some(DevTreeIndexProp {
                index: self.index,
                node: self.node,
                prop,
            });
        }
        None
    }
}

/***********************************/
/***********  Props      ***********/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexPropIter<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node_iter: DevTreeIndexNodeIter<'a, 'i, 'dt>,
    prop_iter: Option<DevTreeIndexNodePropIter<'a, 'i, 'dt>>,
}
impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexPropIter<'a, 'i, 'dt> {
    fn new(index: &'a DevTreeIndex<'i, 'dt>) -> Self {
        Self {
            index,
            node_iter: index.nodes(),
            prop_iter: None,
        }
    }
}

impl FindNext for DevTreeIndexPropIter<'_, '_, '_> {}
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexPropIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexProp<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        // 1. Advance through each node:
        //   a. Advance through each prop and return that prop.
        loop {
            let prop_iter;

            loop {
                if let Some(prop) = &mut self.prop_iter {
                    prop_iter = prop;
                    break;
                }

                match self.node_iter.next() {
                    Some(node) =>  {
                        self.prop_iter = Some(node.props());
                    },
                    None => return None,
                }
            }

            if let Some(prop) = prop_iter.next() {
                return Some(prop);
            }
        }
    }
}

/***********************************/
/***********  Items      ***********/
/***********************************/

#[derive(Clone)]
pub struct DevTreeIndexItemIter<'a, 'i: 'a, 'dt: 'i> {
    pub index: &'a DevTreeIndex<'i, 'dt>,
    node_iter: DevTreeIndexNodeIter<'a, 'i, 'dt>,
    prop_iter: Option<DevTreeIndexNodePropIter<'a, 'i, 'dt>>,
}
impl<'a, 'i: 'a, 'dt: 'i> DevTreeIndexItemIter<'a, 'i, 'dt> {
    fn new(index: &'a DevTreeIndex<'i, 'dt>) -> Self {
        Self {
            index,
            node_iter: index.nodes(),
            prop_iter: None,
        }
    }
}

impl FindNext for DevTreeIndexItemIter<'_, '_, '_> {}
impl<'a, 'i: 'a, 'dt: 'i> Iterator for DevTreeIndexItemIter<'a, 'i, 'dt> {
    type Item = DevTreeIndexItem<'a, 'i, 'dt>;

    fn next(&mut self) -> Option<Self::Item> {
        // 1. Advance through each node:
        //   a. Advance through each prop and return that prop.
        loop {
            let prop_iter;

            loop {
                if let Some(prop) = &mut self.prop_iter {
                    prop_iter = prop;
                    break;
                }

                match self.node_iter.next() {
                    Some(node) =>  {
                        self.prop_iter = Some(node.props());
                        return Some(DevTreeIndexItem::Node(node));
                    },
                    None => return None,
                }
            }

            if let Some(prop) = prop_iter.next() {
                return Some(DevTreeIndexItem::Prop(prop));
            }
        }
    }
}

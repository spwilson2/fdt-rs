//! This module implements a flattened device tree (FDT) index for higher-performanance device tree
//! traversal.

// There are some actions which will take significant amount of time to perform without an index
// built on top of the device tree.
//
// For example, if we want to add a method to parse a node's "reg" property, then we need to know
// the #size-cells and #address-cells of its parent. Currently we don't have a method for storing
// the parent in a DevTreeNode because it would requireda self-referential data structure.
//
// E.g.
// struct DevTreeNode {
//  parent: Option<DevTreeNode>,
// }
//
// We could work around this as we kind of did by specifying an offset:
//
// E.g.
// struct DevTreeNode {
//  parent: Option<NonZeroUsize>,
// }
//
// However using this method it would not be possible to find the parent node's parent.
// Therefore, in order to get a quick parse, we need to index nodes by pointer.
// This way we can traverse the heirarchy in both BFS and DFS (rather than a DFS with inclusion of
// properties).
//
// It seems like we could probably pre-parse the device tree to figure out what the size of the
// buffer pool would be, and then return that value. (It should simply be a component of
// Sum((Node's props) * (size_of::<Prop>) + (Node's nodes) * (size_of::<Node>))
//


// TODO: Semantics for parsing errors.
// (E.g. unwrap)

use crate::*;
use crate::iters;
use core::mem::align_of;

// TODO Add a wrapper around these that is easier to use (that includes a reference to the fdt).
struct DevTreeIndexProp<'dt> {
    propbuf: &'dt [u8],
    nameoff: AssociatedOffset<'dt>,
}

// TODO Add a wrapper around these that is easier to use (that includes a reference to the fdt).
struct DevTreeIndexNode<'dt, 'b> {
    parent: Option<&'b DevTreeIndexNode<'dt, 'b>>,
    children: &'b [DevTreeIndexNode<'dt, 'b>],
    props: &'b [DevTreeIndexProp<'dt>],
    fdt_offset: AssociatedOffset<'dt>,
}

/// Returns the minimum alignment for items stored inside the [`DevTreeIndex`].
const fn idx_elem_align() -> usize {
    let l = align_of::<DevTreeIndexProp>();
    let r = align_of::<DevTreeIndexNode>();
    [l, r][(l > r) as usize]
}

/// TODO
struct DevTreeIndex<'dt, 'b> {
    fdt: DevTree<'dt>,
    buf: &'b mut [u8],
}

impl<'b, 'dt: 'b> DevTreeIndex<'dt, 'b> {

    /// Returns the required size of a buffer used to create a [`DevTreeIndex`] of the given
    /// [`DevTree`].
    pub fn required_size(fdt: &DevTree<'dt>) -> usize {
        // TODO Initial size (base components - e.g. root pointer?)
        let mut size = 0usize;

        // TODO Check based on alignment
        for item in fdt.items() {
            match item {
                DevTreeItem::Node(n) => {
                    size += size_of::<DevTreeIndexNode>();
                },
                DevTreeItem::Prop(p) => {
                    size += size_of::<DevTreeIndexProp>();
                },
            }
        }
        size
    }

    pub unsafe fn new(fdt: DevTree<'dt>, buf: &'b mut [u8]) -> Self {

        // TODO Check Alignment
        assert!(buf.len() >= Self::required_size(&fdt));


        // TODO Custom in-memory stack:
        //


        // Iterate through all elements to the leaf
        // Do a DFS for nodes.
        // - On each visited node set its parent
        // Once we've see an EndNode token, we're done with a node.
        // The current node becomes the parent.
        //let addr = buf.as_mut_ptr() as usize;

        let mut nodes = core::mem::transmute::<_, &'b mut [DevTreeIndexNode<'dt, 'b>]>(buf);
        let mut current_node : Option<&'b DevTreeIndexNode<'dt, 'b>> = None;
        let mut node_idx = 0;

        // Initial discovery:
        //
        // - Parse nodes and properties through DFS, creating indexes as we go along.
        // - On completion of node parse, move the parsed index into the perminent index.
        //
        // As this process continues, memory looks like:
        //
        // ===== Start of Mem ======
        // ===== Temporary mem ======
        // - RootNode
        // - Prop
        // - Prop
        //   - Child Node
        //   - Prop
        //     - Grandchild Node
        //     - Grandchild Node
        //       - Prop
        // - Empty Slot
        // - Empty Slot
        // - Empty Slot
        // ===== Permenent mem ======
        //       - Great Grandchild
        //         - Prop
        //       - Great Grandchild
        //         - Prop
        // ===== End of Memory ======
        //
        // As we finish parsing nodes (we hit ParsedTok::EndNode)
        // We then mem move the parsed grandchild into Permenent mem and resize limits
        let mut buf_addr = AssociatedOffset::new(0, buf);
        let mut fdt_addr = AssociatedOffset::new(fdt.off_dt_struct(), fdt.buf);
        asse

        loop {
            // Safe because we only pass offsets which are returned by next_devtree_token.
            let res = unsafe { iters::next_devtree_token(fdt.buf, &mut fdt_addr) };

            match res {
                Ok(Some(iters::ParsedTok::BeginNode(node))) => {
                    if let Some(n) = current_node {
                        // TODO If current_node add this discovered node to our node list.

                        buf_addr
                    }

                    nodes[node_idx].parent = current_node;

                    // Discard lifetime.
                    //
                    // NOTE: While, technically unsafe; we use this to create an internal reference
                    // while we modify nodes below it. Since this function only operates using
                    // a single thread at a time, we guaruntee safety of taking multiple references
                    // to the mutable node.
                    current_node = Some(&*(&nodes[node_idx] as *const DevTreeIndexNode<'dt, 'b>));

                    // Next node will be inserted after this one..
                    node_idx += 1;
                }
                Ok(Some(iters::ParsedTok::Prop(prop))) => {
                    // TODO? Ignore for now.
                }
                Ok(Some(iters::ParsedTok::EndNode)) => {
                    // TODO Now our node list is complete, shift it over to a perminent position.

                    // Unwrap - fails if we see a property before a node in the device tree.
                    // This would be a bad DTB.
                    current_node = current_node.unwrap().parent;
                }
                Ok(Some(iters::ParsedTok::Nop)) => continue,
                Ok(None) => break,
                _ => panic!("Bad device tree."),
            }
    }

        todo!()
    }
}

extern crate fdt_rs;

use core::mem::size_of;
use fdt_rs::DevTree;
use fdt_rs::fdt_util::props::DevTreePropState;

#[repr(align(4))]
struct _Wrapper<T>(T);
pub const FDT: &[u8] = &_Wrapper(*include_bytes!("riscv64-virt.dtb")).0;

#[test]
fn test_readsize_advice() {
    unsafe {
        let size = DevTree::read_totalsize(FDT).unwrap();
        assert!(size == FDT.len());
        let _blob = DevTree::new(FDT).unwrap();
    }
}

#[test]
fn reserved_entries_iter() {
    unsafe {
        let blob = DevTree::new(FDT).unwrap();
        assert!(blob.reserved_entries().count() == 0);
    }
}

#[test]
fn nodes_iter() {
    unsafe {
        let blob = DevTree::new(FDT).unwrap();
        for node in blob.nodes() {
            let _ = node.name().unwrap();
        }
        assert!(blob.nodes().count() == 27);
    }
}

#[test]
fn node_prop_iter() {
    unsafe {
        let blob = DevTree::new(FDT).unwrap();
        for node in blob.nodes() {
            for prop in node.props() {
                if prop.length() == size_of::<u32>() {
                }
                if prop.length() > 0 {
                    if let Ok(i) = prop.get_str_count() {
                        if i == 0 {
                            continue;
                        }
                        assert!(i < 64);
                        let mut vec: &mut [Option<&str>] = &mut [None; 64];
                        if prop.get_strlist(&mut vec).is_err() {
                            continue;
                        }

                        let mut iter = vec.iter();

                        while let Some(Some(s)) = iter.next() {
                            let _ = s;
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn find_first_compatible_works_on_initial_node() {
    unsafe {
        let fdt = DevTree::new(FDT).unwrap();
        let node = fdt
            .find_first_compatible_node("riscv-virtio")
            .unwrap();
        assert!(node.name().unwrap() == ""); // Root node has no "name"
    }
}

#[test]
fn find_first_compatible_works_on_final_node() {
    unsafe {
        let fdt = DevTree::new(FDT).unwrap();
        let node = fdt
            .find_first_compatible_node("riscv,clint0")
            .unwrap();
        assert!(node.name().unwrap() == "clint@2000000");
    }
}
#[test]
fn find_all_compatible() {
    unsafe {
        let devtree = DevTree::new(FDT).unwrap();
        let compat = "virtio,mmio";
        let exp = "virtio_mmio@1000";
        let mut count = 0;
        let exp_count = 8;

        if let Some(mut cur) = devtree.root() {
            while let Some(node) = cur.find_next_compatible_node(compat) {
                count += 1;
                // Verify the prefix matches.
                // (ascii doesn't have startswith)
                assert!(node.name().unwrap()[0..exp.len()] == *exp);
                cur = node;
                assert!(count <= exp_count);
            }
        }
        assert!(count == exp_count);
    }
}

pub mod alloc_tests {
    use super::*;
    use fdt_rs::index;
    use fdt_rs::index::DevTreeIndex;

    static mut index_buf: &mut[u8] = &mut [0; 50_000];

    struct FdtIndex<'dt> {
        fdt: DevTree<'dt>,
        index: DevTreeIndex<'dt, 'dt>,
    }

    fn get_fdt_index<'dt>() -> FdtIndex<'dt> {
        unsafe {
            let devtree = DevTree::new(FDT).unwrap();
            let layout = index::DevTreeIndex::get_layout(&devtree).unwrap();
            FdtIndex {
                fdt: devtree.clone(),
                index: index::DevTreeIndex::new(devtree, index_buf).unwrap(),
            }
        }
    }

    // Test that we can create an index from a valid device tree
    #[test]
    fn create_index() {
        unsafe {
            let devtree = DevTree::new(FDT).unwrap();
            index::DevTreeIndex::new(devtree, vec![0u8;500000].as_mut_slice()).unwrap();
        }
    }

    // Test that we can create an index from a valid device tree
    #[test]
    fn create_sized_index() {
        unsafe {
            let devtree = DevTree::new(FDT).unwrap();
            let layout = index::DevTreeIndex::get_layout(&devtree).unwrap();
            let mut vec = vec![0u8; layout.size() + layout.align()];
            index::DevTreeIndex::new(devtree, vec.as_mut_slice()).unwrap();
        }
    }

    // Test that an invalid buffer size results in NotEnoughMemory.
    #[test]
    fn expect_create_index_layout_fails_with_invalid_layout() {
        unsafe {
            let devtree = DevTree::new(FDT).unwrap();
            let layout = index::DevTreeIndex::get_layout(&devtree).unwrap();
            let mut vec = vec![0u8; layout.size() - 1];
            index::DevTreeIndex::new(devtree, vec.as_mut_slice()).expect_err("Expected failure.");
        }
    }

    // Test that we can create an index from a valid device tree
    #[test]
    fn dfs_iteration() {
        unsafe {
            let devtree = DevTree::new(FDT).unwrap();
            let mut data = vec![0u8;500000];
            let idx = index::DevTreeIndex::new(devtree, data.as_mut_slice()).unwrap();

            let iter = idx.dfs_iter();
            for n in iter {
                let _ = n.name().unwrap();
            }
        }
    }

    // Test that we can create an index from a valid device tree
    #[test]
    fn root_prop_iteration() {
        unsafe {
            let devtree = DevTree::new(FDT).unwrap();
            let mut data = vec![0u8;500000];
            let idx = index::DevTreeIndex::new(devtree, data.as_mut_slice()).unwrap();

            let iter = idx.dfs_iter();
            for n in iter {
                let _ = n.name().unwrap();
            }
        }
    }
}

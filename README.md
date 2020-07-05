# fdt-rs

A Flattened Device Tree parser for embedded no-std environments

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies.fdt-rs]
version = "0.2"
```

and this to your crate root:

```rust
extern crate fdt_rs;
```

## Features

This crate can be used without the standard library (`#![no_std]`) by disabling
the default `std` feature. Use this in `Cargo.toml`:

```toml
[dependencies.fdt-rs]
version = "0.2"
default-features = false
```

## Example

The following example stashes a flattened device tree in memory, parses that
device tree into a `fdt_rs::DevTree` object, searches the device tree for the
first "ns16550a" compatible node, and if found prints that node's name.

```rust
extern crate fdt_rs;
use fdt_rs::prelude::*;
use fdt_rs::base::*;

// Place a device tree image into the rust binary and
// align it to a 32-byte boundary by using a wrapper struct.
#[repr(align(4))] struct _Wrapper<T>(T);
pub const FDT: &[u8] = &_Wrapper(*include_bytes!("../tests/riscv64-virt.dtb")).0;

fn main() {
    // Initialize the devtree using an &[u8] array.
    let devtree = unsafe {

        // Get the actual size of the device tree after reading its header.
        let size = DevTree::read_totalsize(FDT).unwrap();
        let buf = &FDT[..size];

        // Create the device tree handle
        DevTree::new(buf).unwrap()
    };

    // Find the first "ns16550a" compatible node within the device tree.
    // If found, print the name of that node (including unit address).
    if let Some(node) = devtree.find_first_compatible_node("ns16550a") {
        println!("{}", node.name().unwrap());
    }
}

```

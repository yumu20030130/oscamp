#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;
extern crate alloc;

use alloc::vec::Vec;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    println!("Running bump tests...");

    let mut pool = Vec::new();

    for i in 0.. {
        println!("Indicator: {}", i);
        let mut items = alloc_pass(i);
        free_pass(&mut items);

        pool.append(&mut items);
        assert_eq!(items.len(), 0);
    }

    println!("Bump tests run OK!");
}

fn alloc_pass(delta: usize) -> Vec<Vec<u8>> {
    let mut items = Vec::new();
    let mut base = 32;
    loop {
        let a = vec![0u8; base+delta];
        println!("alloc: {}", a.len());
        items.push(a);
        if base >= 512*1024 {
            break;
        }
        base *= 2;
    }
    items
}

fn free_pass(items: &mut Vec<Vec<u8>>) {
    let total = items.len();
    for j in (0..total).rev() {
        println!("remove {j}");
        if j % 2 == 0 {
            items.remove(j);
        }
    }
}

//! This is a test binary for the kernel

#![allow(async_fn_in_trait)]
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

fn main() {
    // Add some actual code that won't be optimized away
    let mut x = 42u32;
    let mut y = 0u32;

    for i in 0..100 {
        x = x.wrapping_add(i);
        y = y.wrapping_mul(2);

        // Use volatile operations to prevent optimization
        unsafe {
            core::ptr::write_volatile(&mut x, x);
            y = core::ptr::read_volatile(&y);
        }
    }

    // Ensure the values are used
    if x > y {
        loop {
            unsafe {
                core::ptr::write_volatile(&mut x, x.wrapping_add(1));
            }
        }
    } else {
        loop {
            unsafe {
                core::ptr::write_volatile(&mut y, y.wrapping_add(1));
            }
        }
    }
}

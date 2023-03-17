//! This is the kernel for the doors operating system. It is written in rust and pieces of it (as required) are written in assembly.

#![no_std]
#![no_main]
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]

use doors_kernel_api::video::TextDisplay;

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
mod x86;

fn main(vga: &spin::Mutex<impl TextDisplay>) -> ! {
    let mut v = vga.lock();
    v.print_str("Entered main main function\r\n");
    drop(v);
    loop {}
}
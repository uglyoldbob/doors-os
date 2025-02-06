//! Test code for the kernel
#![no_std]
#![no_main]
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(abi_x86_interrupt)]
#![feature(box_vec_non_null)]

doors_macros::load_config!();

extern crate alloc;

doors_macros::use_doors_test!();

mod common;
pub use common::*;

pub mod boot;
pub use boot::mem2::*;
pub mod kernel;
pub mod modules;

pub use boot::IoPortArray;
pub use boot::IoPortManager;
pub use boot::IoPortRef;

pub use modules::video::TextDisplay;
use modules::video::TextDisplayTrait;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "arm")] {
        /// The io port manager
        pub static IO_PORT_MANAGER: Option<&Locked<IoPortManager>> = None;
    } else if #[cfg(any(target_arch = "x86_64", target_arch = "x86"))] {
        /// The io port manager
        pub static IO_PORT_MANAGER: Option<&Locked<IoPortManager>> = Some(&boot::IOPORTS);
    }
}

/// This creates the multiboot2 signature that allows the kernel to be booted by a multiboot compliant bootloader such as grub.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[link_section = ".multiboot"]
#[used]
static MULTIBOOT_HEADER: boot::multiboot::Multiboot = boot::multiboot::Multiboot::new();

use kernel::SystemTrait;

doors_macros::define_doors_test_runner!();

/// The panic handler for the kernel
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    doors_macros2::kernel_print!("PANIC AT THE DISCO!\r\n");
    let msg = info.message();
    if let Some(s) = msg.as_str() {
        doors_macros2::kernel_print_alloc!("{}\r\n", s);
    }
    doors_macros2::kernel_print_alloc!("{}\r\n", info);
    if let Some(t) = info.location() {
        let f = t.file();
        let maxlen = f.len();
        for i in (0..maxlen).step_by(70) {
            let tmax = if i + 70 < maxlen { i + 70 } else { maxlen };
            doors_macros2::kernel_print!("{}\r\n", &f[i..tmax]);
        }
        doors_macros2::kernel_print!(" LINE {}\r\n", t.line());
    }
    doors_macros2::kernel_print!("PANIC SOMEWHERE ELSE!\r\n");
    loop {}
}

fn main(mut system: kernel::System) -> ! {
    {
        system.enable_interrupts();
        system.init();
        doors_macros2::kernel_print!("DoorsOs running tests\r\n");
        match DoorsTester::doors_test_main() {
            Ok(()) => doors_macros2::kernel_print!("All tests passed\r\n"),
            Err(()) => doors_macros2::kernel_print!("At least one test failed\r\n"),
        }
        doors_macros2::kernel_print!("Entering idle loop\r\n");
        loop {
            system.idle();
        }
    }
}

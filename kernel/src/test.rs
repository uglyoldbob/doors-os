//! Test code for the kernel
#![no_std]
#![no_main]
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![feature(allocator_api)]
#![feature(abi_x86_interrupt)]
#![feature(async_fn_traits)]

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
    crate::VGA.print_str("PANIC AT THE DISCO!\r\n");
    if let Some(t) = info.location() {
        let f = t.file();
        let maxlen = f.len();
        for i in (0..maxlen).step_by(70) {
            let tmax = if i + 70 < maxlen { i + 70 } else { maxlen };
            crate::VGA.print_str(&f[i..tmax]);
        }
        crate::VGA.print_str("\r\n");
        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
            " LINE {}\r\n",
            t.line()
        ));
    }
    let msg = info.message();
    if let Some(s) = msg.as_str() {
        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!("{}\r\n", s));
    }
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!("{}\r\n", info));
    crate::VGA.print_str("PANIC SOMEWHERE ELSE!\r\n");
    loop {}
}

fn main(mut system: kernel::System) -> ! {
    {
        if true {
            doors_macros::todo_item_panic!("This should never happen");
        }
        system.enable_interrupts();
        system.init();
        //SYSTEM.replace(Some(system));
        if DoorsTester::doors_test_main().is_err() {
            crate::VGA.print_str("At least one test failed\r\n");
        }
        let mut executor = Executor::default();
        executor.run()
    }
}

doors_macros::populate_todo_list!();

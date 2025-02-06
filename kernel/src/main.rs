//! This is the kernel for the doors operating system. It is written in rust and pieces of it (as required) are written in assembly.

#![no_std]
#![no_main]
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(abi_x86_interrupt)]
#![feature(box_vec_non_null)]
#![feature(custom_test_frameworks)]

extern crate alloc;

doors_macros::use_doors_test!();

mod common;
pub use common::*;

pub mod boot;
pub mod kernel;
pub mod modules;

pub use boot::IoPortArray;
pub use boot::IoPortManager;
pub use boot::IoPortRef;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "arm")] {
        /// The io port manager
        pub static IO_PORT_MANAGER: Option<&Locked<IoPortManager>> = None;
    } else if #[cfg(any(target_arch = "x86_64", target_arch = "x86"))] {
        /// The io port manager
        pub static IO_PORT_MANAGER: Option<&Locked<IoPortManager>> = Some(&boot::IOPORTS);
    }
}

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

use kernel::SystemTrait;
use modules::network::NetworkAdapterTrait;
use modules::rng;
use modules::rng::RngTrait;
use modules::video::hex_dump_generic;
pub use modules::video::TextDisplay;
use modules::video::TextDisplayTrait;

/// This creates the multiboot2 signature that allows the kernel to be booted by a multiboot compliant bootloader such as grub.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[link_section = ".multiboot"]
#[used]
static MULTIBOOT_HEADER: boot::multiboot::Multiboot = boot::multiboot::Multiboot::new();

/// Used to debug some stuff in the kernel
pub static DEBUG_STUFF: Locked<[u32; 82]> = Locked::new([0; 82]);

fn main(mut system: kernel::System) -> ! {
    {
        system.enable_interrupts();
        system.init();
        doors_macros2::kernel_print!("DoorsOs Booting now\r\n");

        {
            doors_macros2::kernel_print!("Registering LFSR rng\r\n");
            let rng = rng::RngLfsr::new();
            kernel::RNGS
                .lock()
                .register_rng(rng::Rng::Lfsr(LockedArc::new(rng)));
        }

        {
            let mut d = kernel::DISPLAYS.lock();
            if d.exists(0) {
                let e = d.module(0);
                let mut f = e.lock();
                if let Some(fb) = f.try_get_pixel_buffer() {
                    doors_macros2::kernel_print!("Writing random data to framebuffer\r\n");
                    let mut rng = kernel::RNGS.lock();
                    let rngm = rng.module(0);
                    let rng = rngm.lock();
                    loop {
                        rng.generate_iter(fb.iter_bytes());
                    }
                }
            }
        }
        {
            if let Some(na) = crate::modules::network::get_network_adapter("net0") {
                let mut na = na.lock();
                doors_macros2::kernel_print!("About to do some stuff with a network card\r\n");
                let ma = na.get_mac_address();
                hex_dump_generic(&ma, false, false);
            }
        }
        doors_macros2::kernel_print!("Entering idle loop\r\n");
        loop {
            system.idle();
        }
    }
}

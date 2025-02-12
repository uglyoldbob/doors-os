//! This is the kernel for the doors operating system. It is written in rust and pieces of it (as required) are written in assembly.

#![no_std]
#![no_main]
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![feature(allocator_api)]
#![feature(abi_x86_interrupt)]
#![feature(async_fn_traits)]
#![feature(type_alias_impl_trait)]
#![feature(unboxed_closures)]

doors_macros::load_config!();

extern crate alloc;

doors_macros::use_doors_test!();

mod common;
use alloc::borrow::ToOwned;
pub use common::*;

pub mod boot;
pub use boot::mem2::*;
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

use kernel::SystemTrait;
use modules::network::NetworkAdapterTrait;
use modules::rng;
use modules::rng::RngTrait;
use modules::video::hex_dump_generic_async;
pub use modules::video::TextDisplay;

/// This creates the multiboot2 signature that allows the kernel to be booted by a multiboot compliant bootloader such as grub.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[link_section = ".multiboot"]
#[used]
static MULTIBOOT_HEADER: boot::multiboot::Multiboot = boot::multiboot::Multiboot::new();

/// Used to debug some stuff in the kernel
pub static DEBUG_STUFF: Locked<[u32; 82]> = Locked::new([0; 82]);

/// Run a network test on the net0 network card, if it exists
async fn net_test() {
    crate::VGA
        .print_str_async("Waiting for network card net0\r\n")
        .await;
    while crate::modules::network::get_network_adapter("net0")
        .await
        .is_none()
    {
        executor::Task::yield_now().await;
    }
    crate::VGA
        .print_str_async("Waiting for first rng\r\n")
        .await;
    while !kernel::RNGS.lock().exists(0) {
        executor::Task::yield_now().await;
    }
    crate::VGA
        .print_str_async("About to do some stuff with a network card net0\r\n")
        .await;
    if let Some(na) = crate::modules::network::get_network_adapter("net0").await {
        let rng = kernel::RNGS.lock().module(0);
        let mut na = na.lock();
        crate::VGA
            .print_str_async("About to do some stuff with a network card\r\n")
            .await;
        let ma = na.get_mac_address();
        hex_dump_generic_async(&ma, false, false).await;
        crate::VGA
            .print_str_async("Done doing stuff with network card\r\n")
            .await;
        for i in 0..32 {
            crate::VGA
                .print_str_async(&alloc::format!("Sending packet {}\r\n", i))
                .await;
            let mut packet = [0; 64];
            {
                let rng = rng.lock();
                rng.generate_iter(packet.iter_mut());
            }
            na.send_packet(&packet).await.unwrap();
        }
    }
}

fn main() -> ! {
    {
        let sys = SYSTEM.sync_lock().to_owned().unwrap();
        sys.enable_interrupts();
        sys.init();
        crate::VGA.print_str("DoorsOs Booting now\r\n");

        {
            let mut d = kernel::DISPLAYS.lock();
            if d.exists(0) {
                let e = d.module(0);
                let mut f = e.lock();
                if let Some(fb) = f.try_get_pixel_buffer() {
                    crate::VGA.print_str("Writing random data to framebuffer\r\n");
                    let mut rng = kernel::RNGS.lock();
                    let rngm = rng.module(0);
                    let rng = rngm.lock();
                    loop {
                        rng.generate_iter(fb.iter_bytes());
                    }
                }
            }
        }
        crate::VGA.print_str("About to start the executor\r\n");
        let mut executor = Executor::default();
        executor
            .spawn_closure(async || {
                crate::VGA.print_str_async("Registering LFSR rng\r\n").await;
                let rng = rng::RngLfsr::new();
                kernel::RNGS
                    .lock()
                    .register_rng(rng::Rng::Lfsr(LockedArc::new(rng)));
            })
            .unwrap();
        executor.spawn(executor::Task::new(net_test())).unwrap();
        executor
            .spawn_closure(async || {
                modules::pci::setup_pci().await;
            })
            .unwrap();
        executor
            .spawn_closure(async || {
                for i in 0..32 {
                    crate::VGA
                        .print_str_async(&alloc::format!("I am groot {}\r\n", i))
                        .await;
                    executor::Task::yield_now().await;
                }
            })
            .unwrap();
        executor
            .spawn_closure(async || {
                for i in 0..32 {
                    crate::VGA
                        .print_str_async(&alloc::format!("I am batman {}\r\n", i))
                        .await;
                    executor::Task::yield_now().await;
                }
            })
            .unwrap();
        executor.run()
    }
}

doors_macros::populate_todo_list!();

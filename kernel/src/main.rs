//! This is the kernel for the doors operating system. It is written in rust and pieces of it (as required) are written in assembly.

#![no_std]
#![no_main]
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![feature(allocator_api)]
#![feature(strict_provenance)]
#![feature(const_mut_refs)]
#![feature(alloc_error_handler)]

extern crate alloc;

pub mod boot;
pub mod kernel;
pub mod modules;

use alloc::boxed::Box;
use doors_kernel_api::video::TextDisplay;

use crate::modules::gpio::GpioTrait;

/// A wrapper structure that allows for a thing to be wrapped with a mutex.
pub struct Locked<A> {
    /// The contained thing
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    /// Create a new protected thing
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    /// Lock the mutex and return a protected instance of the thing
    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }

    /// Replace the contents of the protected instance with another instance of the thing
    pub fn replace(&self, r: A) {
        let mut s = self.inner.lock();
        *s = r;
    }
}

/// This creates the multiboot2 signature that allows the kernel to be booted by a multiboot compliant bootloader such as grub.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[link_section = ".multiboot"]
#[used]
static MULTIBOOT_HEADER: boot::multiboot::Multiboot = boot::multiboot::Multiboot::new();

/// The VGA instance used for x86 kernel printing
static VGA: spin::Mutex<Option<Box<dyn TextDisplay>>> = spin::Mutex::new(None);

fn main() -> ! {
    doors_macros2::kernel_print!("I am groot\r\n");
    {
        let mut gpio = crate::kernel::GPIO.lock();
        let mg = gpio.module(0);
        let mh = gpio.module(9);
        drop(gpio);
        let mut g = mg.lock();
        let mut h = mh.lock();
        g.reset(false);
        h.reset(false);
        g.set_output(12);
        h.set_output(5);
        h.set_output(13);
        loop {
            g.write_output(12, true);
            h.write_output(5, true);
            h.write_output(13, true);

            g.write_output(12, false);
            h.write_output(5, false);
            h.write_output(13, false);
        }
    }
}

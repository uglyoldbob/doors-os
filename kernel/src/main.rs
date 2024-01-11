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

use alloc::sync::Arc;
use doors_kernel_api::video::TextDisplay;

/// A wrapper around box that allows for traits to be implemented on a Box
pub struct Box<T> {
    /// The contained object
    inner: alloc::boxed::Box<T>,
}

impl<T: Clone> Clone for Box<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> core::ops::Deref for Box<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> core::ops::DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// A wrapper that allows for traits to be implemented on an Arc<Mutex<A>>
pub struct LockedArc<A> {
    /// The arc with the contained object
    inner: Arc<Locked<A>>,
}

impl<A> Clone for LockedArc<A> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<A> LockedArc<A> {
    /// Create a new locked arc object.
    pub fn new(inner: A) -> Self {
        Self {
            inner: Arc::new(Locked::new(inner)),
        }
    }

    /// Lock the contained mutex, returning a protected instance of the contained object
    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }

    /// Replace the contents of the protected instance with another instance of the thing
    pub fn replace(&self, r: A) {
        let mut s = self.inner.lock();
        *s = r;
    }
}

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
static VGA: spin::Mutex<Option<alloc::boxed::Box<dyn TextDisplay>>> = spin::Mutex::new(None);

/// Used to debug some stuff in the kernel
pub static DEBUG_STUFF: Locked<[u32; 82]> = Locked::new([0; 82]);

fn main() -> ! {
    doors_macros2::kernel_print!("I am groot\r\n");
    {
        use crate::modules::gpio::GpioTrait;
        use crate::modules::serial::SerialTrait;

        let mut serials = crate::kernel::SERIAL.lock();
        let serial = serials.module(0);
        drop(serials);
        let s = serial.lock();
        s.setup(115200);

        let mut gpio = crate::kernel::GPIO.lock();
        let mg = gpio.module(0);

        let mh = gpio.module(9);
        drop(gpio);
        let mut gpioa = mg.lock();

        let mut h = mh.lock();
        gpioa.reset(false);
        h.reset(false);

        //set the pin for the mco1 clock output
        gpioa.set_alternate(8, 0);
        //set the pins for the uart hardware
        gpioa.set_alternate(9, 7);
        gpioa.set_alternate(10, 7);
        //enable high speed output for the clock output
        gpioa.set_speed(8, 3);

        gpioa.set_output(12);
        h.set_output(5);
        h.set_output(13);
        loop {
            gpioa.write_output(12, true);
            h.write_output(5, true);
            h.write_output(13, true);

            s.sync_transmit_str("i am groot\r\n");

            gpioa.write_output(12, false);
            h.write_output(5, false);
            h.write_output(13, false);
        }
    }
}

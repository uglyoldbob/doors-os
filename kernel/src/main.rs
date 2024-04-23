//! This is the kernel for the doors operating system. It is written in rust and pieces of it (as required) are written in assembly.

#![no_std]
#![no_main]
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![feature(allocator_api)]
#![feature(strict_provenance)]
#![feature(const_mut_refs)]
#![feature(alloc_error_handler)]
#![feature(new_uninit)]
#![feature(abi_x86_interrupt)]

extern crate alloc;

pub mod boot;
pub mod kernel;
pub mod modules;

use alloc::sync::Arc;
use modules::video::TextDisplay;
use modules::video::TextDisplayTrait;

/// A fixed string type that allows for strings of up to 80 characters.
pub type FixedString = arraystring::ArrayString<arraystring::typenum::U80>;

/// Get the address of the specified variable
pub fn address<T>(v: &T) -> usize {
    v as *const T as usize
}

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
static VGA: spin::Mutex<Option<TextDisplay>> = spin::Mutex::new(None);

/// Used to debug some stuff in the kernel
pub static DEBUG_STUFF: Locked<[u32; 82]> = Locked::new([0; 82]);

struct ColorCycler {
    color: u16,
    num_bits: u8,
    shift: u8,
}

impl ColorCycler {
    fn new() -> Self {
        Self {
            color: 1,
            num_bits: 1,
            shift: 0,
        }
    }

    fn get_color(&self) -> u16 {
        self.color
    }

    fn advance(&mut self) {
        if (self.num_bits + self.shift) < 15 {
            self.shift += 1;
        } else {
            self.shift = 0;
            if self.num_bits < 14 {
                self.num_bits += 1;
            } else {
                self.num_bits = 1;
            }
        }

        let ones = 0xffff >> (16 - self.num_bits);
        let val = ones << self.shift;
        self.color = val;
    }
}

fn main() -> ! {
    {
        doors_macros2::kernel_print!("DoorsOs Booting now\r\n");
        loop {}
    }
}

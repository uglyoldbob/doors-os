//! This is the kernel for the doors operating system. It is written in rust and pieces of it (as required) are written in assembly.

#![no_std]
#![no_main]
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(abi_x86_interrupt)]
#![feature(box_vec_non_null)]

extern crate alloc;

pub mod boot;
pub mod kernel;
pub mod modules;

pub use boot::IoPortArray;
pub use boot::IoPortManager;
pub use boot::IoPortRef;

/// The trait that allows reading and writing to and from io ports
pub trait IoReadWrite<T> {
    /// Read data from the io port, with the proper size. It is advised that the address be properly aligned for the size of access being performed.
    fn port_read(&mut self) -> T;
    /// Write data to the io port, with the proper size. It is advised that the address be properly aligned for the size of access being performed.
    fn port_write(&mut self, val: T);
}

/// A struct that manages allocation and deallocation of pci memory
pub struct PciMemory {
    /// The starting address for virtual memory address space
    virt: usize,
    /// The starting address for physical memory address space
    phys: usize,
    /// The size in bytes
    size: usize,
}

impl PciMemory {
    /// Read a u32 at the specified index (byte based index)
    pub fn read_u32(&self, address: usize) -> u32 {
        let mem = unsafe { core::slice::from_raw_parts(self.virt as *const u8, self.size) };
        let a: &u8 = &mem[address];
        let b: *const u8 = a as *const u8;
        let c: &u32 = unsafe { &*(b as *const u32) };
        unsafe { core::ptr::read_volatile(c) }
    }

    /// Write a u32 at the specified index (byte based index), with the specified value
    pub fn write_u32(&mut self, address: usize, val: u32) {
        let mem = unsafe { core::slice::from_raw_parts_mut(self.virt as *mut u8, self.size) };
        let a: &mut u8 = &mut mem[address as usize];
        let b: *mut u8 = a as *mut u8;
        let c: &mut u32 = unsafe { &mut *(b as *mut u32) };
        unsafe { core::ptr::write_volatile(c, val) };
    }
}

/// A structure that generically maps dma memory over a type.
pub struct DmaMemory<T> {
    /// The starting address for virtual memory address space
    virt: usize,
    /// The starting address for physical memory address space
    phys: usize,
    /// The size in bytes
    size: usize,
    /// The data (in virtual memory space)
    data: alloc::boxed::Box<T>,
}

impl<T> core::ops::Deref for DmaMemory<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> core::ops::DerefMut for DmaMemory<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

/// Used to store an array of items for dma
pub struct DmaMemorySlice<T> {
    /// The starting address for virtual memory address space
    virt: usize,
    /// The starting address for physical memory address space
    phys: usize,
    /// The size in bytes
    size: usize,
    /// The data (in virtual memory space)
    data: alloc::vec::Vec<T>,
}

impl<T> core::ops::Deref for DmaMemorySlice<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> core::ops::DerefMut for DmaMemorySlice<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "arm")] {
        /// The io port manager
        pub static IO_PORT_MANAGER: Option<&Locked<IoPortManager>> = None;
    } else if #[cfg(any(target_arch = "x86_64", target_arch = "x86"))] {
        /// The io port manager
        pub static IO_PORT_MANAGER: Option<&Locked<IoPortManager>> = Some(&boot::IOPORTS);
    }
}

use alloc::sync::Arc;
use kernel::SystemTrait;
use modules::rng;
use modules::rng::RngTrait;
use modules::video::TextDisplay;
use modules::video::TextDisplayTrait;

/// A fixed string type that allows for strings of up to 80 characters.
pub type FixedString = arraystring::ArrayString<arraystring::typenum::U80>;

/// Get the address of the specified variable
pub fn address<T>(v: &T) -> usize {
    v as *const T as usize
}

/// The the address of a slice variable
pub fn slice_address<T>(v: &[T]) -> usize {
    v as *const [T] as *const T as usize
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
static VGA: Locked<Option<TextDisplay>> = Locked::new(None);

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
        doors_macros2::kernel_print!("Entering idle loop\r\n");
        loop {}
    }
}

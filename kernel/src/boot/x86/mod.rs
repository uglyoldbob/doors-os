//! The generic x86 module covering both 32 and 64-bit functionality.

use crate::modules::video::text::X86VgaTextMode;
use crate::VGA;
use alloc::boxed::Box;
use doors_kernel_api::FixedString;
use doors_kernel_api::video::TextDisplay;
use lazy_static::lazy_static;

#[cfg(target_arch = "x86_64")]
pub mod boot64;
#[cfg(target_arch = "x86_64")]
pub use boot64 as boot;

#[cfg(target_arch = "x86_64")]
use x86_64::instructions::port::{PortRead, PortWrite};

#[cfg(target_arch = "x86")]
pub mod boot32;
#[cfg(target_arch = "x86")]
pub use boot32 as boot;

pub mod memory;

lazy_static! {
    /// The entire list of io ports for an x86 machine
    pub static ref IOPORTS: spin::Mutex<IoPortManager> =
        spin::Mutex::new(unsafe { IoPortManager::new() });
}

/// The heap for the kernel. This global allocator is responsible for the majority of dynamic memory in the kernel.
#[global_allocator]
static HEAP_MANAGER: crate::Locked<memory::HeapManager> = crate::Locked::new(
    memory::HeapManager::new(&boot::PAGING_MANAGER, &boot::VIRTUAL_MEMORY_ALLOCATOR),
);

/// A reference to a single io port
pub struct IoPortRef {
    /// The address of the io port
    r: u16,
}

/// The trait that allows reading and writing to and from io ports
pub trait IoReadWrite {
    /// Read data from the io port, with the proper size. It is advised that the address be properly aligned for the size of access being performed.
    fn port_read(port: IoPortRef) -> Self;
    /// Write data to the io port, with the proper size. It is advised that the address be properly aligned for the size of access being performed.
    fn port_write(port: IoPortRef, val: Self);
}

impl IoReadWrite for u8 {
    fn port_read(port: IoPortRef) -> Self {
        unsafe {
            #[cfg(target_arch = "x86")]
            return x86::io::inb(port.r);
            #[cfg(target_arch = "x86_64")]
            return Self::read_from_port(port.r);
        }
    }

    fn port_write(port: IoPortRef, val: Self) {
        unsafe {
            #[cfg(target_arch = "x86")]
            x86::io::outb(port.r, val);
            #[cfg(target_arch = "x86_64")]
            Self::read_from_port(port.r);
        }
    }
}

impl IoReadWrite for u16 {
    fn port_read(port: IoPortRef) -> Self {
        unsafe {
            #[cfg(target_arch = "x86")]
            return x86::io::inw(port.r);
            #[cfg(target_arch = "x86_64")]
            return Self::read_from_port(port.r);
        }
    }

    fn port_write(port: IoPortRef, val: Self) {
        unsafe {
            #[cfg(target_arch = "x86")]
            x86::io::outw(port.r, val);
            #[cfg(target_arch = "x86_64")]
            Self::write_to_port(port.r, val);
        }
    }
}

impl IoReadWrite for u32 {
    fn port_read(port: IoPortRef) -> Self {
        unsafe {
            #[cfg(target_arch = "x86")]
            return x86::io::inl(port.r);
            #[cfg(target_arch = "x86_64")]
            return Self::read_from_port(port.r);
        }
    }

    fn port_write(port: IoPortRef, val: Self) {
        unsafe {
            #[cfg(target_arch = "x86")]
            x86::io::outl(port.r, val);
            #[cfg(target_arch = "x86_64")]
            Self::write_to_port(port.r, val);
        }
    }
}

/// An array of io ports.
pub struct IoPortArray {
    /// The first port address of the array.
    base: u16,
    /// The quantity of ports in the array.
    quantity: u16,
}

impl IoPortArray {
    /// Get a port reference, ensuring that it is not out of bounds for the array. Will panic if out of bounds.
    pub fn port(&self, index: u16) -> IoPortRef {
        assert!(index < self.quantity);
        IoPortRef {
            r: self.base + index,
        }
    }
}

/// Keeps track of all io ports on the system.
pub struct IoPortManager {
    ports: [usize; 65536 / core::mem::size_of::<usize>()],
}

impl IoPortManager {
    /// Create a new io port manager. All ports are assumed to be unused initially.
    pub unsafe fn new() -> Self {
        Self {
            ports: [0; 65536 / core::mem::size_of::<usize>()],
        }
    }

    /// Try to get some io ports from the system.
    pub fn get_ports(&mut self, base: u16, quantity: u16) -> Option<IoPortArray> {
        let mut possible = true;
        for p in base..base + quantity {
            let index = p / core::mem::size_of::<usize>() as u16;
            let shift = p % core::mem::size_of::<usize>() as u16;
            let d = self.ports[index as usize] & 1 << shift;
            if d != 0 {
                possible = false;
            }
        }
        if possible {
            for p in base..base + quantity {
                let index = p / core::mem::size_of::<usize>() as u16;
                let shift = p % core::mem::size_of::<usize>() as u16;
                self.ports[index as usize] |= 1 << shift;
            }
            Some(IoPortArray { base, quantity })
        } else {
            None
        }
    }
}

/// This function is called by the entrance module for the kernel.
fn main_boot() -> ! {
    let vga = unsafe { X86VgaTextMode::get(0xb8000) };
    let mut b: alloc::boxed::Box<dyn TextDisplay> = alloc::boxed::Box::new(vga);
    let mut v = VGA.lock();
    v.replace(b);
    drop(v);
    doors_macros2::kernel_print!("This is a test\r\n");
    super::super::main();
}

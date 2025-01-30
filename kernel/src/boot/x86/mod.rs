//! The generic x86 module covering both 32 and 64-bit functionality.

use core::marker::PhantomData;

use crate::modules::pci::PciTrait;
use crate::modules::video::TextDisplayTrait;
use crate::Locked;
use crate::LockedArc;
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

pub use boot::PCI_MEMORY_ALLOCATOR;

pub mod memory;

pub use boot::PciMemoryAllocator;

lazy_static! {
    /// The entire list of io ports for an x86 machine
    pub static ref IOPORTS: Locked<IoPortManager> =
        Locked::new(unsafe { IoPortManager::new() });
}

/// The heap for the kernel. This global allocator is responsible for the majority of dynamic memory in the kernel.
#[global_allocator]
static HEAP_MANAGER: Locked<memory::HeapManager> = Locked::new(memory::HeapManager::new(
    &boot::PAGING_MANAGER,
    &boot::VIRTUAL_MEMORY_ALLOCATOR,
));

/// A reference to a single io port
pub struct IoPortRef<T> {
    /// The address of the io port
    r: u16,
    /// Phantom data that the port contains
    _marker: PhantomData<T>,
}

/// The trait that allows reading and writing to and from io ports
pub trait IoReadWrite<T> {
    /// Read data from the io port, with the proper size. It is advised that the address be properly aligned for the size of access being performed.
    fn port_read(&mut self) -> T;
    /// Write data to the io port, with the proper size. It is advised that the address be properly aligned for the size of access being performed.
    fn port_write(&mut self, val: T);
}

impl IoReadWrite<u8> for IoPortRef<u8> {
    fn port_read(&mut self) -> u8 {
        unsafe {
            #[cfg(target_arch = "x86")]
            return x86::io::inb(self.r);
            #[cfg(target_arch = "x86_64")]
            return u8::read_from_port(self.r);
        }
    }

    fn port_write(&mut self, val: u8) {
        unsafe {
            #[cfg(target_arch = "x86")]
            x86::io::outb(self.r, val);
            #[cfg(target_arch = "x86_64")]
            u8::write_to_port(self.r, val);
        }
    }
}

impl IoReadWrite<u16> for IoPortRef<u16> {
    fn port_read(&mut self) -> u16 {
        unsafe {
            #[cfg(target_arch = "x86")]
            return x86::io::inw(self.r);
            #[cfg(target_arch = "x86_64")]
            return u16::read_from_port(self.r);
        }
    }

    fn port_write(&mut self, val: u16) {
        unsafe {
            #[cfg(target_arch = "x86")]
            x86::io::outw(self.r, val);
            #[cfg(target_arch = "x86_64")]
            u16::write_to_port(self.r, val);
        }
    }
}

impl IoReadWrite<u32> for IoPortRef<u32> {
    fn port_read(&mut self) -> u32 {
        unsafe {
            #[cfg(target_arch = "x86")]
            return x86::io::inl(self.r);
            #[cfg(target_arch = "x86_64")]
            return u32::read_from_port(self.r);
        }
    }

    fn port_write(&mut self, val: u32) {
        unsafe {
            #[cfg(target_arch = "x86")]
            x86::io::outl(self.r, val);
            #[cfg(target_arch = "x86_64")]
            u32::write_to_port(self.r, val);
        }
    }
}

/// An array of io ports.
pub struct IoPortArray<'a> {
    /// The first port address of the array.
    base: u16,
    /// The quantity of ports in the array.
    quantity: u16,
    /// A reference to the ioportmanager
    manager: &'a Locked<IoPortManager>,
}

impl<'a> Drop for IoPortArray<'a> {
    fn drop(&mut self) {
        self.manager.return_ports(self)
    }
}

impl<'a> IoPortArray<'a> {
    /// Get a port reference, ensuring that it is not out of bounds for the array. Will panic if out of bounds.
    pub fn port<T>(&self, index: u16) -> IoPortRef<T> {
        assert!(index < self.quantity);
        IoPortRef {
            r: self.base + index,
            _marker: PhantomData,
        }
    }
}

/// Keeps track of all io ports on the system.
pub struct IoPortManager {
    ports: [usize; 65536 / core::mem::size_of::<usize>()],
}

impl Locked<IoPortManager> {
    /// Try to get a single port from the system
    pub fn get_port<T>(&self, base: u16) -> Option<IoPortRef<T>> {
        let mut manager = self.lock();
        let p = base;
        let index = p / core::mem::size_of::<usize>() as u16;
        let shift = p % core::mem::size_of::<usize>() as u16;
        let d = manager.ports[index as usize] & 1 << shift;
        if d != 0 {
            None
        } else {
            manager.ports[index as usize] |= 1 << shift;
            Some(IoPortRef {
                r: base,
                _marker: PhantomData,
            })
        }
    }

    /// Try to get some io ports from the system.
    pub fn get_ports(&self, base: u16, quantity: u16) -> Option<IoPortArray> {
        let mut manager = self.lock();
        let mut possible = true;
        for p in base..base + quantity {
            let index = p / core::mem::size_of::<usize>() as u16;
            let shift = p % core::mem::size_of::<usize>() as u16;
            let d = manager.ports[index as usize] & 1 << shift;
            if d != 0 {
                possible = false;
            }
        }
        if possible {
            for p in base..base + quantity {
                let index = p / core::mem::size_of::<usize>() as u16;
                let shift = p % core::mem::size_of::<usize>() as u16;
                manager.ports[index as usize] |= 1 << shift;
            }
            Some(IoPortArray {
                base,
                quantity,
                manager: self,
            })
        } else {
            None
        }
    }

    /// Returns a list of port previously obtained fromm the manager
    fn return_ports(&self, ports: &mut IoPortArray) {
        let mut manager = self.lock();
        for p in ports.base..ports.base + ports.quantity {
            let index = p / core::mem::size_of::<usize>() as u16;
            let shift = p % core::mem::size_of::<usize>() as u16;
            manager.ports[index as usize] &= !(1 << shift);
        }
    }
}

impl IoPortManager {
    /// Create a new io port manager. All ports are assumed to be unused initially.
    pub unsafe fn new() -> Self {
        Self {
            ports: [0; 65536 / core::mem::size_of::<usize>()],
        }
    }
}

extern "C" {
    /// Defines the start of the kernel for initial kernel load. This is defined by the linker script.
    pub static START_OF_KERNEL: u8;
    /// Defines the end of the kernel for the initial kernel load. This is defined by the linker script.
    pub static END_OF_KERNEL: u8;
}

/// This function is called by the entrance module for the kernel.
fn main_boot(mut system: crate::kernel::System) -> ! {
    {
        let mut serials = crate::kernel::SERIAL.lock();
        for base in [0x3f8, 0x2f8, 0x3e8, 0x2e8, 0x5f8, 0x4f8, 0x5e8, 0x4e8] {
            if let Some(com) = crate::modules::serial::x86::X86SerialPort::new(base) {
                doors_macros2::kernel_print!("Registered serial port {:x}\r\n", base);
                let com = crate::modules::serial::Serial::PcComPort(LockedArc::new(com));
                crate::modules::serial::SerialTrait::sync_transmit_str(
                    &com,
                    "Serial port test\r\n",
                );
                serials.register_serial(com);
            }
        }

        if serials.exists(0) {
            let s = serials.module(0);
            let sd = s.make_text_display();
            let mut v = crate::VGA.lock();
            v.replace(sd);
            drop(v);
        }
    }

    doors_macros2::kernel_print!("This is a test\r\n");
    {
        let pci = crate::modules::pci::x86::Pci::new();
        if let Some(pci) = pci {
            let mut pci = crate::modules::pci::Pci::X86Pci(pci);
            pci.setup();
            crate::modules::pci::pci_register_drivers();
            pci.driver_setup(&mut system);
        }
    }
    {
        let h = HEAP_MANAGER.lock();
        h.print();
    }
    super::super::main(system);
}

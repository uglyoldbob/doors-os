//! The generic x86 module covering both 32 and 64-bit functionality.

use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use crate::modules::serial::SerialTrait;
use crate::Arc;
use crate::AsyncLockedArc;
use crate::IoReadWrite;
use crate::Locked;

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

pub use boot::mem2;

/// The entire list of io ports for an x86 machine
pub static IOPORTS: Locked<IoPortManager> = Locked::new(IoPortManager::new());

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

impl Drop for IoPortArray<'_> {
    fn drop(&mut self) {
        self.manager.return_ports(self)
    }
}

impl IoPortArray<'_> {
    /// Get a port reference, ensuring that it is not out of bounds for the array. Will panic if out of bounds.
    pub fn port<T>(&self, index: u16) -> IoPortRef<T> {
        doors_macros::todo_item!("Figure out how to disallow port writes on the port ref for this");
        assert!(index < self.quantity);
        IoPortRef {
            r: self.base + index,
            _marker: PhantomData,
        }
    }

    /// Get the base address of the io address array
    pub fn get_base(&self) -> u16 {
        self.base
    }
}

/// Keeps track of all io ports on the system.
pub struct IoPortManager {
    /// A bitmap to track usage of all the ports for an x86 system
    ports: [usize; 65536 / core::mem::size_of::<usize>()],
}

impl Locked<IoPortManager> {
    /// Try to get a single port from the system
    pub fn get_port<T>(&self, base: u16) -> Option<IoPortRef<T>> {
        let mut manager = self.sync_lock();
        let p = base;
        let index = p / core::mem::size_of::<usize>() as u16;
        let shift = p % core::mem::size_of::<usize>() as u16;
        let d = manager.ports[index as usize] & (1 << shift);
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
        let mut manager = self.sync_lock();
        let mut possible = true;
        for p in base..base + quantity {
            let index = p / core::mem::size_of::<usize>() as u16;
            let shift = p % core::mem::size_of::<usize>() as u16;
            let d = manager.ports[index as usize] & (1 << shift);
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
        let mut manager = self.sync_lock();
        for p in ports.base..ports.base + ports.quantity {
            let index = p / core::mem::size_of::<usize>() as u16;
            let shift = p % core::mem::size_of::<usize>() as u16;
            manager.ports[index as usize] &= !(1 << shift);
        }
    }
}

impl Default for IoPortManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IoPortManager {
    /// Create a new io port manager. All ports are assumed to be unused initially.
    pub const fn new() -> Self {
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

/// Probe and setup all serial ports for x86
/// This will probably be removed once pci space is further developed
fn setup_serial() {
    let mut serials = crate::kernel::SERIAL.sync_lock();
    for (base, irq) in [(0x3f8, 4), (0x2f8, 3), (0x3e8, 4), (0x2e8, 3)] {
        if let Some(com) = crate::modules::serial::x86::X86SerialPort::new(base, irq) {
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "Registered serial port {:x}\r\n",
                base
            ));
            let com = crate::modules::serial::Serial::PcComPort(com);
            use crate::modules::serial::SerialTrait;
            for i in 0..100 {
                com.sync_transmit_str(&alloc::format!("Testing the serial port {}\r\n", i));
            }
            serials.register_serial(com);
        }
    }
}

/// Enable interrupts for the first serial port if it is present
fn serial_interrupts() {
    let sys = crate::SYSTEM.read().clone();
    if let Some(mut s) = crate::kernel::SERIAL.take_device(0) {
        s.sync_transmit_str("About to enable async mode for serial port 0\r\n");
        s.enable_async(sys.clone()).unwrap();
        s.sync_flush();
        let t = s.convert(
            |a| a.make_text_display(),
            move |t| {
                todo!();
            },
        );
        crate::common::VGA.sync_replace(Some(t));
    }
    if let Some(mut s) = crate::kernel::SERIAL.take_device(1) {
        s.enable_async(sys.clone()).unwrap();
    }
}

/// This function is called by the entrance module for the kernel.
fn main_boot() -> ! {
    crate::main()
}

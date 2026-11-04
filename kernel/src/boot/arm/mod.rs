//! Boot code for arm based platforms

use crate::Locked;

use crate::modules::video::TextDisplayTrait;

/// Attempt to get a str from a panic payload
pub fn get_panic_message(panic: &dyn core::any::Any) -> Option<&str> {
    panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().map(core::ops::Deref::deref))
}

/// A reference to a single io port
pub struct IoPortRef<T> {
    /// The address of the io port
    r: u32,
    /// Phantom data that the port contains
    _marker: core::marker::PhantomData<T>,
}

use crate::IoReadWrite;

impl IoReadWrite<u8> for IoPortRef<u8> {
    fn port_read(&mut self) -> u8 {
        unsafe { core::ptr::read_volatile(self.r as *const u8) }
    }

    fn port_write(&mut self, val: u8) {
        unsafe { core::ptr::write_volatile(self.r as *mut u8, val) }
    }
}

impl IoReadWrite<u16> for IoPortRef<u16> {
    fn port_read(&mut self) -> u16 {
        unsafe { core::ptr::read_volatile(self.r as *const u16) }
    }

    fn port_write(&mut self, val: u16) {
        unsafe { core::ptr::write_volatile(self.r as *mut u16, val) }
    }
}

impl IoReadWrite<u32> for IoPortRef<u32> {
    fn port_read(&mut self) -> u32 {
        unsafe { core::ptr::read_volatile(self.r as *const u32) }
    }

    fn port_write(&mut self, val: u32) {
        unsafe { core::ptr::write_volatile(self.r as *mut u32, val) }
    }
}

/// An array of io ports.
pub struct IoPortArray<'a> {
    /// The first port address of the array.
    base: u32,
    /// The quantity of ports in the array.
    quantity: u32,
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
    pub fn port<T>(&self, index: u32) -> IoPortRef<T> {
        doors_macros::todo_item!("Figure out how to disallow port writes on the port ref for this");
        assert!(index < self.quantity);
        IoPortRef {
            r: self.base + index,
            _marker: core::marker::PhantomData,
        }
    }

    /// Get the base address of the io address array
    pub fn get_base(&self) -> u32 {
        self.base
    }
}

/// 'Doesn't' Keep track of all io ports on the system.
pub struct IoPortManager {}

impl Locked<IoPortManager> {
    /// Try to get a single port from the system
    pub fn get_port<T>(&self, base: u32) -> Option<IoPortRef<T>> {
        Some(IoPortRef {
            r: base,
            _marker: core::marker::PhantomData,
        })
    }

    /// Try to get some io ports from the system.
    pub fn get_ports(&self, base: u32, quantity: u32) -> Option<IoPortArray<'_>> {
        Some(IoPortArray {
            base,
            quantity,
            manager: self,
        })
    }

    /// Returns a list of port previously obtained fromm the manager
    fn return_ports(&self, ports: &mut IoPortArray) {}
}

impl Default for IoPortManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IoPortManager {
    /// Create a new io port manager. All ports are assumed to be unused initially.
    pub const fn new() -> Self {
        Self {}
    }
}

/// The heap for the kernel. This global allocator is responsible for the majority of dynamic memory in the kernel.
#[global_allocator]
static HEAP_MANAGER: Locked<hardware::memory::HeapManager> =
    Locked::new(hardware::memory::HeapManager::new());

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769i_disco;

use alloc::string::String;
#[cfg(kernel_machine = "stm32f769i-disco")]
pub use stm32f769i_disco as hardware;

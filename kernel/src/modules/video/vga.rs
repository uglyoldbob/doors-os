//! Kernel module for x86 vga text using video mode

use crate::boot::x86::IOPORTS;
use crate::boot::x86::IoPortArray;

/// The memory portion of the x86 hardware
pub struct X86VgaHardware {
    /// The actual memory
    buf: [volatile::Volatile<u8>; 0x200000],
}

/// The structure for vga hardware operated in plain text mode.
pub struct X86VgaMode<'a> {
    /// The column where the next character will be placed
    column: u8,
    /// The row where the next character will be placed
    row: u8,
    /// A mutable reference to the hardware memory
    hw: &'a mut X86VgaHardware,
    /// The io ports for the vga hardware
    ports: IoPortArray,
}

impl<'a> X86VgaMode<'a> {
    /// Gets an instance of the X86Vga. This should be protected by a singleton type pattern to prevent multiple instances from being handed out to the kernel.
    pub unsafe fn get(adr: usize, base: u16) -> Option<Self> {
        let mut io = IOPORTS.lock();
        let ports = io.get_ports(base, 16).unwrap();
        drop(io);
        Some(Self {
            hw: &mut *(adr as *mut X86VgaHardware),
            column: 0,
            row: 0,
            ports,
        })
    }
}
//! Kernel module for x86 vga text using video mode

use crate::VGA;
use doors_kernel_api::FixedString;
use doors_kernel_api::video::TextDisplay;

use crate::boot::x86::IoPortArray;
use crate::boot::x86::IOPORTS;

/// The memory portion of the x86 hardware
pub struct X86VgaHardware {
    /// The actual memory
    buf: [volatile::Volatile<u8>; 0x40000],
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
    ports: IoPortArray<'a>,
}

impl<'a> X86VgaMode<'a> {
    /// Gets an instance of the X86Vga. This should be protected by a singleton type pattern to prevent multiple instances from being handed out to the kernel.
    pub unsafe fn get(adr: usize, base: u16) -> Option<Self> {
        let ports = IOPORTS.get_ports(base, 16).unwrap();
        let mut check = Self {
            hw: &mut *(adr as *mut X86VgaHardware),
            column: 0,
            row: 0,
            ports,
        };

        Some(check)
    }

    /// Detect how much memory is present on the graphics card
    pub fn detect_memory(&mut self) -> usize {
        const MULTIPLE : usize = 32768;
        for i in (0..self.hw.buf.len()).step_by(MULTIPLE)
        {
            doors_macros2::kernel_print!("Checking {:x}\r\n", i);
        }
        42
    }
}

impl <'a> TextDisplay for X86VgaMode<'a> {
    fn print_char(&mut self, d: char) {
    }
}
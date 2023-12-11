//! Kernel module for x86 vga text using video mode

use crate::VGA;
use doors_kernel_api::video::TextDisplay;
use doors_kernel_api::FixedString;

use crate::boot::x86::IoPortArray;
use crate::boot::x86::IoPortRef;
use crate::boot::x86::IoReadWrite;
use crate::boot::x86::IOPORTS;

/// The memory portion of the x86 hardware
pub struct X86VgaHardware {
    /// The actual memory
    buf: [u8; 0x40000],
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
        let mut pr: IoPortRef<u8> = check.ports.port(0xc);
        let pv: u8 = pr.port_read();
        let mut pw: IoPortRef<u8> = check.ports.port(2);
        pw.port_write(pv | 2);

        let mut pw: IoPortRef<u8> = check.ports.port(6);
        doors_macros2::kernel_print!("mm of VGA is {:x}\r\n", pw.port_read());
        pw.port_write(0);

        let mut pw: IoPortRef<u8> = check.ports.port(4);
        doors_macros2::kernel_print!("extmem of VGA is {:x}\r\n", pw.port_read());
        let p = pw.port_read();
        pw.port_write(p | 2);

        Some(check)
    }

    /// Detect how much memory is present on the graphics card
    pub fn detect_memory(&mut self) -> usize {
        const MULTIPLE: usize = 32768;
        let mut ramsize = 0;
        for i in (0..self.hw.buf.len()).step_by(MULTIPLE) {
            doors_macros2::kernel_print!("Checking {:x}\r\n", i);
            self.hw.buf[i] = 0;
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            self.hw.buf[i + 1] = 1;
            core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
            let val = self.hw.buf[i];
            doors_macros2::kernel_print!("Val is {} at {:p}\r\n", val, &self.hw.buf[i]);
            let good = val == 0;
            if !good {
                break;
            } else {
                ramsize = i + MULTIPLE;
            }
        }
        ramsize
    }
}

impl<'a> TextDisplay for X86VgaMode<'a> {
    fn print_char(&mut self, d: char) {}
}

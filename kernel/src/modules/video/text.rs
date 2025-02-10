//! The kernel module for x86 vga text mode.

use super::VgaChar;

/// The memory in use for the x86 vga hardware
pub struct X86VgaTextHardware {
    /// The vga screen buffer as implemented in hardware
    buf: [[volatile::Volatile<VgaChar>; 80]; 25],
}

/// Structure for the vga mode of standard computers
pub struct X86VgaTextMode {
    /// The column where the next character will be placed
    column: u8,
    /// The row where the next character will be placed
    row: u8,
    /// A mutable reference to the hardware memory
    hw: &'static mut X86VgaTextHardware,
}

impl X86VgaTextMode {
    /// Gets an instance of the X86Vga. This should be protected by a singleton type pattern to prevent multiple instances from being handed out to the kernel.
    /// # Safety
    /// This should be called in a manner to ensure that duplicates are not created
    pub unsafe fn get(adr: usize) -> Self {
        Self {
            hw: &mut *(adr as *mut X86VgaTextHardware),
            column: 0,
            row: 0,
        }
    }
}

impl crate::modules::video::TextDisplayTrait for X86VgaTextMode {
    fn print_char(&mut self, d: char) {
        let p = if d.is_ascii() { d as u8 } else { b'?' };
        match d {
            '\r' => {
                self.column = 0;
            }
            '\n' => {
                if self.row < 24 {
                    self.row += 1;
                } else {
                    for i in 0..24 {
                        for j in 0..80 {
                            self.hw.buf[i][j].write(self.hw.buf[i + 1][j].read());
                        }
                    }
                    for i in 0..80 {
                        self.hw.buf[24][i].write(VgaChar {
                            ascii: b' ',
                            color: 0x0f,
                        });
                    }
                }
            }
            _ => {
                self.hw.buf[self.row as usize][self.column as usize].write(VgaChar {
                    ascii: p,
                    color: 0x0f,
                });
                self.column += 1;
                if self.column >= 80 {
                    self.column = 0;
                    self.row += 1;
                }
                if self.row >= 25 {
                    self.row = 0;
                }
            }
        }
    }

    async fn print_char_async(&mut self, d: char) {
        self.print_char(d);
    }

    async fn print_str_async(&mut self, d: &str) {
        self.print_str(d);
    }

    async fn flush(&mut self) {}
}

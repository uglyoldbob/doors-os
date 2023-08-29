//! The generic x86 module covering both 32 and 64-bit functionality.

use doors_kernel_api::video::TextDisplay;
use lazy_static::lazy_static;

#[cfg(target_arch = "x86_64")]
pub mod boot64;
#[cfg(target_arch = "x86_64")]
pub use boot64 as boot;

#[cfg(target_arch = "x86")]
pub mod boot32;
#[cfg(target_arch = "x86")]
pub use boot32 as boot;

pub mod memory;

/// Type used for the pc vga text mode output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct VgaChar {
    /// The ascii character to print to the screen
    ascii: u8,
    /// The foreground and background color for the character
    color: u8,
}

impl core::ops::Deref for VgaChar {
    type Target = Self;
    fn deref(&self) -> &Self::Target {
        self
    }
}

impl core::ops::DerefMut for VgaChar {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}

/// The memory in use for the x86 vga hardware
pub struct X86VgaHardware {
    /// The vga screen buffer as implemented in hardware
    buf: [[volatile::Volatile<VgaChar>; 80]; 25],
}

/// Structure for the vga mode of standard computers
pub struct X86Vga<'a> {
    /// The column where the next character will be placed
    column: u8,
    /// The row where the next character will be placed
    row: u8,
    /// A mutable reference to the hardware memory
    hw: &'a mut X86VgaHardware,
}

impl<'a> X86Vga<'a> {
    /// Gets an instance of the X86Vga. This should be protected by a singleton type pattern to prevent multiple instances from being handed out to the kernel.
    unsafe fn get(adr: usize) -> Self {
        unsafe {
            X86Vga {
                hw: &mut *(adr as *mut X86VgaHardware),
                column: 0,
                row: 0,
            }
        }
    }
}

impl<'a> doors_kernel_api::video::TextDisplay for X86Vga<'a> {
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
                            self.hw.buf[i][j].write(self.hw.buf[i+1][j].read());
                        }
                    }
                    for i in 0..80 {
                        self.hw.buf[24][i].write(VgaChar { ascii: b' ', color: 0x0f});
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
}

lazy_static! {
    /// The VGA instance used for x86 kernel printing
    static ref VGA: spin::Mutex<X86Vga<'static>> =
        spin::Mutex::new(unsafe { X86Vga::get(0xb8000) });
    static ref IOPORTS: spin::Mutex<bitarray::BitArray<65536>> =
        spin::Mutex::new(bitarray::BitArray::new([0; 65536]));
}

/// The heap for the kernel. This global allocator is responsible for the majority of dynamic memory in the kernel.
#[global_allocator]
static HEAP_MANAGER: crate::Locked<memory::HeapManager> = crate::Locked::new(
    memory::HeapManager::new(&boot::PAGING_MANAGER, &boot::VIRTUAL_MEMORY_ALLOCATOR),
);

/// This function is called by the entrance module for the kernel.
fn main_boot() -> ! {
    VGA.lock().print_str("main boot\r\n");
    super::main(&*VGA);
}


use doors_kernel_api::{video::TextDisplay, FixedString};
use lazy_static::lazy_static;

#[cfg(target_arch = "x86_64")]
mod boot64;
#[cfg(target_arch = "x86_64")]
use boot64 as boot;

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
                if self.row < 25 {
                    self.row += 1;
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

#[cfg(target_arch = "x86")]
mod boot32;
#[cfg(target_arch = "x86")]
use boot32 as boot;

/// Scans for the location of the RSDP structure, returning a Some if it is found.
pub fn scan_for_rsdp() -> Option<usize> {
    let ebda: &u16 = unsafe { &*(0x40e as *const &u16) };
    let table = unsafe {&*((*ebda * 0x10) as *const [u8; 0x20000])};

    /// This is the string to search for that identifies the start of the RSDP data
    const MATCH: [u8; 8] = [b'R', b'S', b'D', b' ', b'P', b'T', b'R', b' '];
    for i in 0..0x20000 / 16 {
        if table[16 * i..] == MATCH {
            return Some(0x80000 + 16 * i);
        }
    }
    let table: &[u8; 0x20000] = unsafe { &*(0xE0000 as *const [u8; 0x20000]) };
    for i in 0..0x20000 / 16 {
        if table[16 * i..] == MATCH {
            return Some(0xe0000 + 16 * i);
        }
    }
    None
}

/// This function is called by the entrance module for the kernel.
fn main_boot() -> ! {
    if let Some(_a) = scan_for_rsdp() {
        let mut b = VGA.lock();
        b.print_str("Found the RSDP\r\n");
    } else {
        VGA.lock().print_str("Did not find the RSDP\r\n");
    }
    if let Some(_a) = scan_for_rsdp() {
        let mut b = VGA.lock();
        b.print_str("Found the RSDPp\r\n");
    } else {
        VGA.lock().print_str("Did not find the RSDPp\r\n");
    }

    VGA.lock().print_str("main boot\r\n");
    super::main(&*VGA);
}

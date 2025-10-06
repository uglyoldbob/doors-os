//! Interrupt descriptor table code for x86

/// The interrupt descriptor table for x86
#[repr(C, align(16))]
pub struct InterruptDescriptorTable {
    /// The actual table
    entries: [[u32; 2]; 256],
}

impl InterruptDescriptorTable {
    /// Produces an empty table
    pub const fn new() -> Self {
        Self {
            entries: [[0; 2]; 256],
        }
    }

    /// Set an interrupt handler that takes no arguments
    pub unsafe fn set_handler(&mut self, index: u8, f: extern "x86-interrupt" fn(u32, u32)) {
        let faddr = f as *const () as u32;
        let flow = faddr & 0xFFFF;
        let fhigh = faddr & 0xFFFF0000;
        let flags = 0b1_00_01110_000_00000;
        let ss = 0x8 << 16;
        self.entries[index as usize][0] = flow | ss;
        self.entries[index as usize][1] = fhigh | flags;
    }

    /// Set an interrupt handler that takes no arguments
    pub unsafe fn set_handler_without_arg(&mut self, index: u8, f: extern "x86-interrupt" fn(u32)) {
        let faddr = f as *const () as u32;
        let flow = faddr & 0xFFFF;
        let fhigh = faddr & 0xFFFF0000;
        let flags = 0b1_00_01110_000_00000;
        let ss = 0x8 << 16;
        self.entries[index as usize][0] = flow | ss;
        self.entries[index as usize][1] = fhigh | flags;
    }

    /// Load the interrupt descriptor table
    pub unsafe fn load_unsafe(&self) {
        let idtp = x86::dtables::DescriptorTablePointer {
            limit: 8 * 256 - 1,
            base: self as *const Self,
        };
        x86::dtables::lidt(&idtp);
    }
}

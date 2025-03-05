//! Interrupt descriptor table code for x86

/// The interrupt descriptor table for x86
#[repr(C, align(16))]
pub struct InterruptDescriptorTable {
    /// The actual table
    entries: [u64; 256],
    /// The highest entry, used for setting the idt limit
    highest: u16,
}

impl InterruptDescriptorTable {
    pub const fn new() -> Self {
        Self {
            entries: [0; 256],
            highest: 0,
        }
    }

    /// Set an interrupt handler that takes no arguments
    pub unsafe fn set_handler(&mut self, index: u8, f: extern "C" fn()) {
        match index {
            0..=7 | 16 | 18 | 19 | 20 | 28 => {
                let faddr = f as *const() as u32;
                self.entries[index as usize] = f as u64;
            }
            _ => {
                panic!("Invalid handler index");
            }
        }
        self.highest = core::cmp::max(self.highest, index as u16 + 1);
    }

    /// Load the interrupt descriptor table
    pub unsafe fn load_unsafe(&self) {
        let idtp = x86::dtables::DescriptorTablePointer {
            limit: 8 * self.highest - 1,
            base: self as *const Self,
        };
        x86::dtables::lidt(&idtp);
    }
}
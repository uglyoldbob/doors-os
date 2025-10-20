//! This module is for code that directly handles interrupt mechanisms.

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
/// The io apic for x86
pub struct IoApic {
    reg_sel: &'static mut u8,
    data: &'static mut u32,
}

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
impl IoApic {
    fn read_register(&mut self, index: u8) -> u32 {
        *self.reg_sel = index;
        *self.data
    }

    /// Construct a new module wit the specified base address for registers
    pub fn new(addr: usize) -> Self {
        let mut s = Self {
            reg_sel: unsafe { &mut *(addr as *mut u8) },
            data: unsafe { &mut *((addr + 0x10) as *mut u32) },
        };
        crate::VGA.print_str(&alloc::format!(
            "IOAPIC ID is {:x}\r\n",
            s.read_register(0) >> 24
        ));
        for i in 0..3 {
            crate::VGA.print_str(&alloc::format!("IOAPIC {:x}\r\n", i));
            crate::VGA.print_str(&alloc::format!("\t{:x}\r\n", s.read_register(i as u8)));
        }
        for i in 16..64 {
            crate::VGA.print_str(&alloc::format!("IOAPIC {:x}\r\n", i));
            crate::VGA.print_str(&alloc::format!("\t{:x}\r\n", s.read_register(i as u8)));
        }
        s
    }
}

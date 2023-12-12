//! The reset controller for the stm32f769

struct Registers {
    regs: [u32; 37],
}

/// The reset and control hardware
pub struct Module<'a> {
    registers: &'a mut Registers,
}

impl<'a> Module<'a> {
    /// Create a new peripheral with the specified address
    pub unsafe fn new(addr: u32) -> Self {
        Self {
            registers: &mut *(addr as *mut Registers),
        }
    }

    /// Enable the specified peripheral
    pub fn enable_peripheral(&mut self, i: u8) -> u32 {
        let n = unsafe { core::ptr::read_volatile(&self.registers.regs[12]) } | 1 << i;
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[12], n) }
        unsafe { core::ptr::read_volatile(&self.registers.regs[12]) }
    }
}

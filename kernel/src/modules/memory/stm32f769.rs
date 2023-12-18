//! The flash memory controller for the stm32f769 processor.

/// The registers for the peripheral.
pub struct Registers {
    regs: [u32; 10],
}

/// The flash memory controller for the stm32f769
pub struct Fmc {
    /// The memory mapped register set for the hardware
    regs: &'static mut Registers,
}

impl Fmc {
    /// Create a new object
    pub unsafe fn new(addr: usize) -> Self {
        Self {
            regs: &mut *(addr as *mut Registers),
        }
    }

    /// Set the number of wait states for the memory controller
    pub fn set_wait_states(&mut self, v: u8) {
        let d = unsafe { core::ptr::read_volatile(&self.regs.regs[0]) } & !0xF;
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[0], d | (v as u32 & 0xF)) };
        unsafe { core::ptr::read_volatile(&self.regs.regs[0]) };
    }
}

//! Power code for the stm32f769

/// The power module registers for the stm32f769
struct PowerRegisters {
    regs: [u32; 4],
}

/// The power module for the stm32f769
pub struct Power {
    regs: &'static mut PowerRegisters,
}

impl Power {
    /// Create a new object
    pub unsafe fn new(addr: usize) -> Self {
        Self {
            regs: &mut *(addr as *mut PowerRegisters),
        }
    }
}

//! Serial code for the stm32f769

/// The register set for the usart hardware
struct UsartRegisters {
    regs: [u32; 11],
}

/// A serial port on the stm32f769
pub struct Usart {
    regs: &'static mut UsartRegisters,
}

impl Usart {
    /// Create an instance of the usart hardware
    pub unsafe fn new(addr: u32) -> Self {
        Self {
            regs: &mut *(addr as *mut UsartRegisters),
        }
    }
}

impl super::SerialTrait for Usart {
    fn setup(&self) {

    }
}
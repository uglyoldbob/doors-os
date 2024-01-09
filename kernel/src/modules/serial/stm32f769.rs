//! Serial code for the stm32f769

/// The register set for the usart hardware
struct UsartRegisters {
    regs: [u32; 11],
}

/// A serial port on the stm32f769
pub struct Usart {
    /// The registers for the hardware
    regs: &'static mut UsartRegisters,
    /// The clock line for this hardware
    clock: crate::modules::clock::ClockRef,
}

impl Usart {
    /// Create an instance of the usart hardware
    pub unsafe fn new(addr: u32, c: crate::modules::clock::ClockRef) -> Self {
        Self {
            regs: &mut *(addr as *mut UsartRegisters),
            clock: c,
        }
    }
}

impl super::SerialTrait for Usart {
    fn setup(&self) {
        use crate::modules::clock::ClockRefTrait;
        self.clock.enable_clock();
    }
}

//! Dsi related code for the stm32f769

use alloc::sync::Arc;

use crate::modules::clock::ClockProviderTrait;

struct LtdcRegisters {
    regs: [u8; 5],
}

struct Ltdc {
    cc: Arc<crate::modules::clock::ClockProvider>,
    regs: &'static mut LtdcRegisters,
}

impl Ltdc {
    pub unsafe fn new(cc: &Arc<crate::modules::clock::ClockProvider>, addr: usize) -> Self {
        Self {
            cc: cc.clone(),
            regs: &mut *(addr as *mut LtdcRegisters),
        }
    }

    pub fn enable(&self) {
        self.cc.enable(4 * 32 + 26);
    }

    pub fn disable(&self) {
        self.cc.disable(4 * 32 + 26);
    }
}

struct DsiRegisters {
    regs: [u8; 5],
}

/// The dsi hardware implementation
pub struct Module {
    /// The hardware for enabling and disabling the clock
    cc: Arc<crate::modules::clock::ClockProvider>,
    /// The registers for the hardware
    regs: &'static mut DsiRegisters,
    /// The related ltdc hardware
    ltdc: Ltdc,
}

impl super::MipiDsiTrait for Module {
    fn enable(&self) {
        self.cc.enable(4 * 32 + 27);
        self.ltdc.enable();
    }

    fn disable(&self) {
        self.ltdc.disable();
        self.cc.disable(4 * 32 + 27);
    }
}

impl Module {
    /// Create a new hardware instance
    pub unsafe fn new(cc: &Arc<crate::modules::clock::ClockProvider>, addr: usize) -> Self {
        Self {
            cc: cc.clone(),
            regs: &mut *(addr as *mut DsiRegisters),
            ltdc: Ltdc::new(cc, 0x4001_6800),
        }
    }
}

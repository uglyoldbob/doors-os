//! Timer modules for the stm32f769

use crate::LockedArc;

struct Registers {
    regs: [u32; 16],
}

/// The basic timer module
pub struct Timer {
    regs: &'static mut Registers,
}

impl Timer {
    /// Build a new timer
    pub unsafe fn new(addr: u32) -> Self {
        Self {
            regs: &mut *(addr as *mut Registers),
        }
    }
}

impl super::TimerTrait for LockedArc<Timer> {
    fn delay_ms(&self, _ms: u32) {

    }
}
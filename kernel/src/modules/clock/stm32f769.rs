//! Clock providers for the stm32f769i-disco board

use alloc::sync::Arc;

use crate::LockedArc;

/// The external oscillator for the stm32f769
pub struct ExternalOscillator {
    /// The hardware for configuring the oscillator
    rcc: LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
    /// The frequency specified for the oscillator, specified in hertz
    frequency: u32,
}

impl ExternalOscillator {
    /// Create a new external oscillator with the specified frequency
    pub unsafe fn new(
        frequency: u32,
        rcc: &LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
    ) -> Self {
        let mut s = Self {
            rcc: rcc.clone(),
            frequency,
        };
        let mut rcc = s.rcc.lock();
        rcc.set_hse(true);
        drop(rcc);
        s
    }
}

impl super::ClockProviderTrait for LockedArc<ExternalOscillator> {
    fn enable(&self, _i: usize) {
        let s = self.lock();
        let mut rcc = s.rcc.lock();
        rcc.set_hse(true);
        drop(rcc);
    }

    fn disable(&self, _i: usize) {
        let s = self.lock();
        let mut rcc = s.rcc.lock();
        rcc.set_hse(false);
        drop(rcc);
    }

    fn is_ready(&self, i: usize) -> bool {
        let mut ready;
        let s = self.lock();
        let mut rcc = s.rcc.lock();
        ready = rcc.hse_ready();
        drop(rcc);
        ready
    }
}

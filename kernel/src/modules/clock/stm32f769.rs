//! Clock providers for the stm32f769i-disco board

use super::ClockRefTrait;
use crate::LockedArc;

/// The external oscillator for the stm32f769
#[derive(Clone)]
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
        Self {
            rcc: rcc.clone(),
            frequency,
        }
    }
}

impl super::ClockProviderTrait for ExternalOscillator {
    fn enable(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_hse(true);
    }

    fn disable(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_hse(false);
    }

    fn is_ready(&self, _i: usize) -> bool {
        let mut rcc = self.rcc.lock();
        rcc.hse_ready()
    }

    fn frequency(&self, _i: usize) -> Option<u32> {
        Some(self.frequency)
    }
}

/// The internal oscillator for the stm32f769
#[derive(Clone)]
pub struct InternalOscillator {
    /// The hardware for configuring the oscillator
    rcc: LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
    /// The frequency specified for the oscillator, specified in hertz
    frequency: u32,
}

impl InternalOscillator {
    /// Create a new internal oscillator with the specified frequency
    pub unsafe fn new(
        frequency: u32,
        rcc: &LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
    ) -> Self {
        Self {
            rcc: rcc.clone(),
            frequency,
        }
    }
}

impl super::ClockProviderTrait for InternalOscillator {
    fn enable(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_hsi(true);
    }

    fn disable(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_hsi(false);
    }

    fn is_ready(&self, _i: usize) -> bool {
        let mut rcc = self.rcc.lock();
        rcc.hsi_ready()
    }

    fn frequency(&self, _i: usize) -> Option<u32> {
        Some(self.frequency)
    }
}

#[derive(Clone)]
/// This mux selects the input for the main pll and the i2s pll of the stm32f769
pub struct Mux1 {
    /// The hardware for configuring
    rcc: LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
    /// The two clock providers
    clocks: [alloc::boxed::Box<super::ClockRef>; 2],
}

impl Mux1 {
    /// Create a new mux
    pub fn new(
        rcc: &LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
        clocks: [alloc::boxed::Box<super::ClockRef>; 2],
    ) -> Self {
        Self {
            rcc: rcc.clone(),
            clocks,
        }
    }
}

impl super::ClockRefTrait for Mux1 {
    fn enable(&self) {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux1();
        let i = if v { 1 } else { 0 };
        self.clocks[i].enable();
    }

    fn disable(&self) {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux1();
        let i = if v { 1 } else { 0 };
        self.clocks[i].disable();
    }

    fn is_ready(&self) -> bool {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux1();
        let i = if v { 1 } else { 0 };
        self.clocks[i].is_ready()
    }

    fn frequency(&self) -> Option<u32> {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux1();
        let i = if v { 1 } else { 0 };
        self.clocks[i].frequency()
    }
}

impl super::ClockMuxTrait for Mux1 {
    fn select(&self, i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_mux1(i > 0);
    }
}

/// The input clock divider for the the main, i2s, and sai pll
#[derive(Clone)]
pub struct Divider1 {
    /// The hardware for configuring
    rcc: LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
    /// The input clock for the divider
    iclk: alloc::boxed::Box<super::ClockRef>,
}

impl Divider1 {
    /// Construct a new divider
    pub fn new(
        rcc: &LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
        iclk: super::ClockRef,
    ) -> Self {
        Self {
            rcc: rcc.clone(),
            iclk: alloc::boxed::Box::new(iclk),
        }
    }

    /// Set the divider
    pub fn set_divider(&self, d: u32) {
        let mut rcc = self.rcc.lock();
        rcc.set_divider1(d);
    }
}

impl super::ClockRefTrait for Divider1 {
    fn frequency(&self) -> Option<u32> {
        let rcc = self.rcc.lock();
        let divider = rcc.get_divider1();
        if let Some(f) = self.iclk.frequency() {
            Some(f / divider)
        } else {
            None
        }
    }

    fn is_ready(&self) -> bool {
        true
    }

    fn enable(&self) {}

    fn disable(&self) {}
}

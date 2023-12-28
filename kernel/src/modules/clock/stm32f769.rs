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
    fn enable_clock(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_hse(true);
    }

    fn disable_clock(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_hse(false);
    }

    fn clock_is_ready(&self, _i: usize) -> bool {
        let rcc = self.rcc.lock();
        rcc.hse_ready()
    }

    fn clock_frequency(&self, _i: usize) -> Option<u64> {
        Some(self.frequency as u64)
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
    fn enable_clock(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_hsi(true);
    }

    fn disable_clock(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_hsi(false);
    }

    fn clock_is_ready(&self, _i: usize) -> bool {
        let mut rcc = self.rcc.lock();
        rcc.hsi_ready()
    }

    fn clock_frequency(&self, _i: usize) -> Option<u64> {
        Some(self.frequency as u64)
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

    fn frequency(&self) -> Option<u64> {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux1();
        drop(rcc);
        let i = if v { 1 } else { 0 };
        self.clocks[i].frequency().map(|f| f as u64)
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
    /// TODO impose the frequency limit at runtime
    pub fn set_divider(&self, d: u32) {
        let mut rcc = self.rcc.lock();
        rcc.set_divider1(d);
    }
}

impl super::ClockRefTrait for Divider1 {
    fn frequency(&self) -> Option<u64> {
        let rcc = self.rcc.lock();
        let fin = rcc.get_divider1();
        drop(rcc);
        self.iclk.frequency().map(|f| f as u64 / fin as u64)
    }

    fn is_ready(&self) -> bool {
        true
    }

    fn enable(&self) {}

    fn disable(&self) {}
}

/// The main pll for the stm32f769
#[derive(Clone)]
pub struct PllMain {
    /// The hardware for configuring
    rcc: LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
    /// The input clock
    iclk: alloc::boxed::Box<super::ClockRef>,
}

impl super::ClockProviderTrait for PllMain {
    fn enable_clock(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_main_pll(true);
    }

    fn disable_clock(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_main_pll(false);
    }

    fn clock_is_ready(&self, _i: usize) -> bool {
        let rcc = self.rcc.lock();
        rcc.main_pll_locked()
    }

    fn clock_frequency(&self, i: usize) -> Option<u64> {
        let vco = self
            .iclk
            .frequency()
            .map(|f| f as u64 * self.get_multiplier() as u64);
        let div = super::PllProviderTrait::get_post_divider(self, i) as u64;
        vco.map(|f| f / div as u64)
    }
}

impl PllMain {
    /// Create a new pll
    pub fn new(
        rcc: &LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
        iclk: super::ClockRef,
    ) -> Self {
        Self {
            rcc: rcc.clone(),
            iclk: alloc::boxed::Box::new(iclk),
        }
    }

    /// Set the multiplier for the pll
    fn set_multiplier(&self, m: u32) {
        let mut rcc = self.rcc.lock();
        rcc.set_multiplier1(m);
    }

    /// Get the multiplier for the pll
    fn get_multiplier(&self) -> u32 {
        let rcc = self.rcc.lock();
        rcc.get_multiplier1()
    }
}

impl super::PllProviderTrait for PllMain {
    fn get_input_frequency(&self) -> Option<u64> {
        self.iclk.frequency()
    }

    fn set_input_divider(&self, d: u32) -> Result<(), super::PllDividerErr> {
        if d != 1 {
            return Err(super::PllDividerErr::ImpossibleDivisor);
        }
        Ok(())
    }

    fn set_post_divider(&self, i: usize, d: u32) -> Result<u32, super::PllDividerErr> {
        let mut rcc = self.rcc.lock();
        rcc.set_main_pll_divisor(i, d as u8);
        Ok(rcc.get_main_pll_divisor(i) as u32)
    }

    fn get_post_divider(&self, i: usize) -> u32 {
        let rcc = self.rcc.lock();
        rcc.get_main_pll_divisor(i) as u32
    }

    fn set_vco_frequency(&self, f: u64) -> Result<(), super::PllVcoSetError> {
        if (100_000_000..=432_000_000).contains(&f) {
            if let Some(fin) = self.iclk.frequency() {
                let multiplier = f / fin;
                if (50..433).contains(&multiplier) {
                    self.set_multiplier(multiplier as u32);
                    Ok(())
                } else {
                    Err(super::PllVcoSetError::CannotHitFrequency)
                }
            } else {
                Err(super::PllVcoSetError::UnknownInputFrequency)
            }
        } else {
            Err(super::PllVcoSetError::FrequencyOutOfRange)
        }
    }
}

/// The mux for the SYSCLK
pub struct MuxSysClk {
    /// The hardware for configuring
    rcc: LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
    /// The clock providers
    clocks: [alloc::boxed::Box<super::ClockRef>; 3],
}

impl MuxSysClk {
    /// Create a new mux
    pub fn new(
        rcc: &LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
        clocks: [alloc::boxed::Box<super::ClockRef>; 3],
    ) -> Self {
        Self {
            rcc: rcc.clone(),
            clocks,
        }
    }
}

impl super::ClockRefTrait for MuxSysClk {
    fn enable(&self) {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux_sysclk();
        self.clocks[v as usize].enable();
    }

    fn disable(&self) {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux_sysclk();
        self.clocks[v as usize].disable();
    }

    fn is_ready(&self) -> bool {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux_sysclk();
        self.clocks[v as usize].is_ready()
    }

    fn frequency(&self) -> Option<u64> {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux_sysclk();
        drop(rcc);
        self.clocks[v as usize].frequency().map(|f| f as u64)
    }
}

impl super::ClockMuxTrait for MuxSysClk {
    fn select(&self, i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_mux_sysclk(i as u8);
    }
}

/// The second pll of the stm32f769, provides clocks for i2s
#[derive(Clone)]
pub struct PllTwo {
    /// The hardware for configuring
    rcc: LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
    /// The input clock
    iclk: alloc::boxed::Box<super::ClockRef>,
}

impl PllTwo {
    /// Create a new pll
    pub fn new(
        rcc: &LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
        iclk: super::ClockRef,
    ) -> Self {
        Self {
            rcc: rcc.clone(),
            iclk: alloc::boxed::Box::new(iclk),
        }
    }

    /// Set the multiplier for the pll
    fn set_multiplier(&self, m: u32) {
        let mut rcc = self.rcc.lock();
        rcc.set_multiplier2(m);
    }

    /// Get the multiplier for the pll
    fn get_multiplier(&self) -> u32 {
        let rcc = self.rcc.lock();
        rcc.get_multiplier2()
    }
}

impl super::ClockProviderTrait for PllTwo {
    fn enable_clock(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_second_pll(true);
    }

    fn disable_clock(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_second_pll(false);
    }

    fn clock_is_ready(&self, _i: usize) -> bool {
        let rcc = self.rcc.lock();
        rcc.second_pll_locked()
    }

    fn clock_frequency(&self, i: usize) -> Option<u64> {
        let vco = self
            .iclk
            .frequency()
            .map(|f| f as u64 * self.get_multiplier() as u64);
        let div = super::PllProviderTrait::get_post_divider(self, i) as u64;
        vco.map(|f| f / div as u64)
    }
}

impl super::PllProviderTrait for PllTwo {
    fn get_input_frequency(&self) -> Option<u64> {
        self.iclk.frequency()
    }

    fn set_input_divider(&self, d: u32) -> Result<(), super::PllDividerErr> {
        if d != 1 {
            return Err(super::PllDividerErr::ImpossibleDivisor);
        }
        Ok(())
    }

    fn set_post_divider(&self, i: usize, d: u32) -> Result<u32, super::PllDividerErr> {
        let mut rcc = self.rcc.lock();
        rcc.set_second_pll_divisor(i, d as u8);
        Ok(rcc.get_second_pll_divisor(i) as u32)
    }

    fn get_post_divider(&self, i: usize) -> u32 {
        let rcc = self.rcc.lock();
        rcc.get_second_pll_divisor(i) as u32
    }

    fn set_vco_frequency(&self, f: u64) -> Result<(), super::PllVcoSetError> {
        if (100_000_000..=432_000_000).contains(&f) {
            if let Some(fin) = self.iclk.frequency() {
                let multiplier = f / fin;
                if (50..433).contains(&multiplier) {
                    self.set_multiplier(multiplier as u32);
                    Ok(())
                } else {
                    Err(super::PllVcoSetError::CannotHitFrequency)
                }
            } else {
                Err(super::PllVcoSetError::UnknownInputFrequency)
            }
        } else {
            Err(super::PllVcoSetError::FrequencyOutOfRange)
        }
    }
}

/// The third pll of the stm32f769, provides clocks for sai2 and the lcd hardware
#[derive(Clone)]
pub struct PllThree {
    /// The hardware for configuring
    rcc: LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
    /// The input clock
    iclk: alloc::boxed::Box<super::ClockRef>,
}

impl PllThree {
    /// Create a new pll
    pub fn new(
        rcc: &LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
        iclk: super::ClockRef,
    ) -> Self {
        Self {
            rcc: rcc.clone(),
            iclk: alloc::boxed::Box::new(iclk),
        }
    }

    /// Set the multiplier for the pll
    fn set_multiplier(&self, m: u32) {
        let mut rcc = self.rcc.lock();
        rcc.set_multiplier3(m);
    }

    /// Get the multiplier for the pll
    fn get_multiplier(&self) -> u32 {
        let rcc = self.rcc.lock();
        rcc.get_multiplier3()
    }
}

impl super::ClockProviderTrait for PllThree {
    fn enable_clock(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_third_pll(true);
    }

    fn disable_clock(&self, _i: usize) {
        let mut rcc = self.rcc.lock();
        rcc.set_third_pll(false);
    }

    fn clock_is_ready(&self, _i: usize) -> bool {
        let rcc = self.rcc.lock();
        rcc.third_pll_locked()
    }

    fn clock_frequency(&self, i: usize) -> Option<u64> {
        let vco = self
            .iclk
            .frequency()
            .map(|f| f as u64 * self.get_multiplier() as u64);
        let div = super::PllProviderTrait::get_post_divider(self, i) as u64;
        vco.map(|f| f / div as u64)
    }
}

impl super::PllProviderTrait for PllThree {
    fn get_input_frequency(&self) -> Option<u64> {
        self.iclk.frequency()
    }

    fn set_input_divider(&self, d: u32) -> Result<(), super::PllDividerErr> {
        if d != 1 {
            return Err(super::PllDividerErr::ImpossibleDivisor);
        }
        Ok(())
    }

    fn set_post_divider(&self, i: usize, d: u32) -> Result<u32, super::PllDividerErr> {
        let mut rcc = self.rcc.lock();
        rcc.set_third_pll_divisor(i, d as u8);
        Ok(rcc.get_third_pll_divisor(i) as u32)
    }

    fn get_post_divider(&self, i: usize) -> u32 {
        let rcc = self.rcc.lock();
        rcc.get_third_pll_divisor(i) as u32
    }

    fn set_vco_frequency(&self, f: u64) -> Result<(), super::PllVcoSetError> {
        if (100_000_000..=432_000_000).contains(&f) {
            if let Some(fin) = self.iclk.frequency() {
                let multiplier = f / fin;
                if (50..433).contains(&multiplier) {
                    self.set_multiplier(multiplier as u32);
                    Ok(())
                } else {
                    Err(super::PllVcoSetError::CannotHitFrequency)
                }
            } else {
                Err(super::PllVcoSetError::UnknownInputFrequency)
            }
        } else {
            Err(super::PllVcoSetError::FrequencyOutOfRange)
        }
    }
}

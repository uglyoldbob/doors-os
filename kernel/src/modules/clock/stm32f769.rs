//! Clock providers for the stm32f769i-disco board

use alloc::boxed::Box;

use super::ClockRefTrait;
use crate::{modules::reset::ResetProviderTrait, LockedArc};

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
    fn enable_clock(&self) {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux1();
        let i = if v { 1 } else { 0 };
        self.clocks[i].enable_clock();
    }

    fn disable_clock(&self) {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux1();
        let i = if v { 1 } else { 0 };
        self.clocks[i].disable_clock();
    }

    fn clock_is_ready(&self) -> bool {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux1();
        let i = if v { 1 } else { 0 };
        self.clocks[i].clock_is_ready()
    }

    fn clock_frequency(&self) -> Option<u64> {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux1();
        drop(rcc);
        let i = if v { 1 } else { 0 };
        self.clocks[i].clock_frequency()
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
    fn clock_frequency(&self) -> Option<u64> {
        let rcc = self.rcc.lock();
        let fin = rcc.get_divider1();
        drop(rcc);
        self.iclk.clock_frequency().map(|f| f as u64 / fin as u64)
    }

    fn clock_is_ready(&self) -> bool {
        true
    }

    fn enable_clock(&self) {}

    fn disable_clock(&self) {}
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
            .clock_frequency()
            .map(|f| f as u64 * self.get_multiplier() as u64);
        let div = super::PllTrait::get_post_divider(self, i) as u64;
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

impl super::PllTrait for PllMain {
    fn get_input_frequency(&self) -> Option<u64> {
        self.iclk.clock_frequency()
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
            if let Some(fin) = self.iclk.clock_frequency() {
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
    fn enable_clock(&self) {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux_sysclk();
        self.clocks[v as usize].enable_clock();
    }

    fn disable_clock(&self) {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux_sysclk();
        self.clocks[v as usize].disable_clock();
    }

    fn clock_is_ready(&self) -> bool {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux_sysclk();
        self.clocks[v as usize].clock_is_ready()
    }

    fn clock_frequency(&self) -> Option<u64> {
        let rcc = self.rcc.lock();
        let v = rcc.get_mux_sysclk();
        drop(rcc);
        self.clocks[v as usize].clock_frequency().map(|f| f as u64)
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
            .clock_frequency()
            .map(|f| f as u64 * self.get_multiplier() as u64);
        let div = super::PllTrait::get_post_divider(self, i) as u64;
        vco.map(|f| f / div as u64)
    }
}

impl super::PllTrait for PllTwo {
    fn get_input_frequency(&self) -> Option<u64> {
        self.iclk.clock_frequency()
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
            if let Some(fin) = self.iclk.clock_frequency() {
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
            .clock_frequency()
            .map(|f| f as u64 * self.get_multiplier() as u64);
        let div = super::PllTrait::get_post_divider(self, i) as u64;
        vco.map(|f| f / div as u64)
    }
}

impl super::PllTrait for PllThree {
    fn get_input_frequency(&self) -> Option<u64> {
        self.iclk.clock_frequency()
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
            if let Some(fin) = self.iclk.clock_frequency() {
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

/// The clock tree provider for the stm32f769
#[derive(Clone)]
pub struct ClockTree {
    /// The external frequency of the low frequency oscillator
    osc32: Box<crate::modules::clock::ClockRef>,
    /// The external frequency of the high frequency oscillator
    oscmain: Box<crate::modules::clock::ClockRef>,
    /// The internal frequency of the main rc oscillator
    oscint: Box<crate::modules::clock::ClockRef>,
    /// The internal frequency of the main low frequency oscillator
    osc32int: Box<crate::modules::clock::ClockRef>,
    /// The hardware for configuring
    rcc: LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
    /// The mux for the main pll
    mux1: super::ClockMux,
    /// The main pll
    pllmain: super::Pll,
}

impl ClockTree {
    /// Construct a new clock tree
    pub fn new(
        osc32: crate::modules::clock::ClockRef,
        oscmain: crate::modules::clock::ClockRef,
        oscint: crate::modules::clock::ClockRef,
        osc32int: crate::modules::clock::ClockRef,
        rcc: LockedArc<crate::modules::reset::stm32f769::Module<'static>>,
    ) -> Self {
        let mux1 = Mux1::new(&rcc, [Box::new(oscint.clone()), Box::new(oscmain.clone())]);
        let mux1 = super::ClockMux::Stm32f769Mux1(mux1);
        let d1 = Divider1::new(&rcc, super::ClockRef::Mux(mux1.clone()));
        let d1 = super::ClockRef::Stm32f769MainDivider(d1);
        Self {
            osc32: Box::new(osc32),
            oscmain: Box::new(oscmain),
            oscint: Box::new(oscint),
            osc32int: Box::new(osc32int),
            rcc: rcc.clone(),
            mux1: mux1,
            pllmain: super::Pll::Stm32f769MainPll(PllMain::new(&rcc, d1)),
        }
    }
}

impl super::PllProviderTrait for crate::LockedArc<ClockTree> {
    fn run_closure(&self, i: u8, c: &dyn Fn(&mut super::Pll)) {
        let mut s = self.lock();
        match i {
            0 => {
                c(&mut s.pllmain);
            }
            _ => {
                panic!("Invalid pll");
            }
        }
    }
}

impl super::ClockProviderTrait for crate::LockedArc<ClockTree> {
    fn disable_clock(&self, i: usize) {
        let mut s = self.lock();
        let mut rcc = s.rcc.lock();
        let d = i / 32;
        let dr = i % 32;
        match (d, dr) {
            (0, 0) => {
                rcc.set_hse(false);
            }
            (0, 1) => {
                rcc.set_hsi(false);
            }
            _ => panic!("Invalid clock specified"),
        }
    }

    fn enable_clock(&self, i: usize) {
        let mut s = self.lock();
        let mut rcc = s.rcc.lock();
        let d = i / 32;
        let dr = i % 32;
        match (d, dr) {
            (0, 0) => {
                rcc.set_hse(true);
            }
            (0, 1) => {
                rcc.set_hsi(true);
            }
            _ => panic!("Invalid clock specified"),
        }
    }

    fn clock_is_ready(&self, i: usize) -> bool {
        let mut s = self.lock();
        let mut rcc = s.rcc.lock();
        let d = i / 32;
        let dr = i % 32;
        match (d, dr) {
            (0, 0) => rcc.hse_ready(),
            (0, 1) => rcc.hsi_ready(),
            _ => panic!("Invalid clock specified"),
        }
    }

    fn clock_frequency(&self, i: usize) -> Option<u64> {
        let mut s = self.lock();
        let d = i / 32;
        let dr = i % 32;
        match (d, dr) {
            (0, 0) => s.oscmain.clock_frequency(),
            (0, 1) => s.oscint.clock_frequency(),
            _ => panic!("Invalid clock specified"),
        }
    }
}

//! Defines clock providers and how clocks are managed in the kernel.

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

use alloc::boxed::Box;

use crate::LockedArc;

/// The trait that all clock providers must implement
#[enum_dispatch::enum_dispatch]
pub trait ClockProviderTrait {
    /// Enable the specified clock
    fn enable(&self, i: usize);
    /// Disable the specified clock
    fn disable(&self, i: usize);
    /// Is the specified clock ready?
    fn is_ready(&self, i: usize) -> bool;
    /// What is the frequency of the clock (if it is known)
    fn frequency(&self, i: usize) -> Option<u32>;
}

/// An enumeration of all the types of gpio controllers
#[derive(Clone)]
#[enum_dispatch::enum_dispatch(ClockProviderTrait)]
pub enum ClockProvider {
    /// The reset provider for the stm32f769i-disco board.
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(LockedArc<crate::modules::reset::stm32f769::Module<'static>>),
    /// The external oscillator for the stm32f769 processor
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769Hse(stm32f769::ExternalOscillator),
    /// The internal oscillator for the smt32f769 processor
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769Hsi(stm32f769::InternalOscillator),
    /// A fake clock provider
    Dummy(DummyClock),
}

impl ClockProvider {
    /// Get a `ClockRef` object from the provider
    pub fn get_ref(&self, i: usize) -> ClockRef {
        ClockRef::Plain(ClockRefPlain {
            clock_provider: self.clone(),
            index: i,
        })
    }
}

/// A fake clock provider
#[derive(Clone)]
pub struct DummyClock {}

impl ClockProviderTrait for DummyClock {
    fn disable(&self, _i: usize) {}

    fn enable(&self, _i: usize) {}

    fn is_ready(&self, _i: usize) -> bool {
        true
    }

    fn frequency(&self, _i: usize) -> Option<u32> {
        None
    }
}

/// The trait for a single clock
#[enum_dispatch::enum_dispatch]
pub trait ClockRefTrait {
    /// Get the frequency of the clock, if known
    fn frequency(&self) -> Option<u32>;
    /// Is the clock ready
    fn is_ready(&self) -> bool;
    /// Enable the clock, if possible
    fn enable(&self);
    /// Disable the clock, if possible
    fn disable(&self);
}

/// A reference to a single clock
#[derive(Clone)]
#[enum_dispatch::enum_dispatch(ClockRefTrait)]
pub enum ClockRef {
    /// A regular reference directly to a clock provider
    Plain(ClockRefPlain),
    /// A clock from a mux
    Mux(ClockMux),
}

/// A clock reference to a single clock.
#[derive(Clone)]
pub struct ClockRefPlain {
    /// The provider of the clock
    clock_provider: ClockProvider,
    /// The index of the clock for the provider
    index: usize,
}

impl ClockRefTrait for ClockRefPlain {
    fn frequency(&self) -> Option<u32> {
        self.clock_provider.frequency(self.index)
    }

    fn is_ready(&self) -> bool {
        self.clock_provider.is_ready(self.index)
    }

    fn enable(&self) {
        self.clock_provider.enable(self.index);
    }

    fn disable(&self) {
        self.clock_provider.disable(self.index);
    }
}

impl ClockRefTrait for DummyClock {
    fn frequency(&self) -> Option<u32> {
        None
    }

    fn is_ready(&self) -> bool {
        true
    }

    fn enable(&self) {}

    fn disable(&self) {}
}

/// The trait for clock mux devices
#[enum_dispatch::enum_dispatch]
pub trait ClockMuxTrait: ClockRefTrait {
    /// Select which clock is to be used
    fn select(&self, i: usize);
}

/// An enumeration of all the types of gpio controllers
#[derive(Clone)]
#[enum_dispatch::enum_dispatch(ClockRefTrait, ClockMuxTrait)]
pub enum ClockMux {
    /// A do nothing clock multiplexer
    DummyMux(DummyClock),
    /// The mux for the main pll and i2s pll of the stm32f769
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769Mux1(stm32f769::Mux1),
}

impl ClockMuxTrait for DummyClock {
    fn select(&self, _i: usize) {}
}

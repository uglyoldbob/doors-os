//! Defines clock providers and how clocks are managed in the kernel.

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

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
    fn frequency(&self, i: usize) -> Option<u64>;
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
    /// The main pll for the stm32f769 processor
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32F769MainPll(stm32f769::PllMain),
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

impl DummyClock {
    /// Construct a dummy clock
    pub fn new() -> Self {
        Self {}
    }
}

impl ClockProviderTrait for DummyClock {
    fn disable(&self, _i: usize) {}

    fn enable(&self, _i: usize) {}

    fn is_ready(&self, _i: usize) -> bool {
        true
    }

    fn frequency(&self, _i: usize) -> Option<u64> {
        None
    }
}

/// The trait for a single clock
#[enum_dispatch::enum_dispatch]
pub trait ClockRefTrait {
    /// Get the frequency of the clock, if known
    fn frequency(&self) -> Option<u64>;
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
    /// The input clock divider for the the main, i2s, and sai pll
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769MainDivider(stm32f769::Divider1),
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
    fn frequency(&self) -> Option<u64> {
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
    fn frequency(&self) -> Option<u64> {
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

/// The trait for clock dividers
#[enum_dispatch::enum_dispatch]
pub trait ClockDividerTrait {
    /// Set the divisor for the object
    fn set_divisor(&self, d: usize) -> Result<(), ()>;
}

/// The trait common to all pll providers
#[enum_dispatch::enum_dispatch]
pub trait PllProviderTrait: ClockProviderTrait {
    /// Set the input frequency of the pll
    fn set_internal_input_frequency(&self, f: u64) -> Result<(), PllDividerErr> {
        if let Some(fin) = self.get_input_frequency() {
            let divider = fin / f;
            self.set_input_divider(divider as u32)
        } else {
            Err(PllDividerErr::UnknownInputFrequency)
        }
    }
    /// Get the input frequency
    fn get_input_frequency(&self) -> Option<u64>;
    /// Set the input divider for the pll
    fn set_input_divider(&self, d: u32) -> Result<(), PllDividerErr>;
    /// Set the post divider
    fn set_post_divider(&self, i: usize, d: u32) -> Result<u32, PllDividerErr>;
    /// Get the post divider
    fn get_post_divider(&self, i: usize) -> u32;
    /// Set the vco frequency of the pll
    fn set_vco_frequency(&self, f: u64) -> Result<(), PllVcoSetError>;
}

/// An enumeration of all the types of pll providers
#[derive(Clone)]
#[enum_dispatch::enum_dispatch(ClockProviderTrait, PllProviderTrait)]
pub enum PllProvider {
    /// The main pll for the stm32f769
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769MainPll(stm32f769::PllMain),
    /// The dsi pll for the stm32f769
    Stm32f769DsiPll(crate::modules::video::mipi_dsi::stm32f769::Module),
    /// A dummy pll provider
    Dummy(DummyClock),
}

impl PllProviderTrait for DummyClock {
    fn get_input_frequency(&self) -> Option<u64> {
        None
    }

    fn set_input_divider(&self, _d: u32) -> Result<(), PllDividerErr> {
        Ok(())
    }

    fn set_post_divider(&self, _i: usize, _d: u32) -> Result<u32, PllDividerErr> {
        Ok(1)
    }

    fn get_post_divider(&self, _i: usize) -> u32 {
        1
    }

    fn set_vco_frequency(&self, _f: u64) -> Result<(), PllVcoSetError> {
        Ok(())
    }
}

/// Errors that can occur setting the input divider of a pll
pub enum PllDividerErr {
    /// The divisor is not possible
    ImpossibleDivisor,
    /// The input frequency is unknown and the the internal input frequency cannot be set
    UnknownInputFrequency,
    /// The input frequency to the divider is out of range
    InputFrequencyOutOfRange,
}

/// Potential errors for setting pll vco frequency
pub enum PllVcoSetError {
    /// The frequency requested for the vco is out of range for the vco
    FrequencyOutOfRange,
    /// The input frequency is unknown
    UnknownInputFrequency,
    /// The pll cannot hit the desired frequency due to adjustment limits
    CannotHitFrequency,
    /// The input frequency is out of range
    InputFrequencyOutOfRange,
}

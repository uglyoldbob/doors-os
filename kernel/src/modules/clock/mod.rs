//! Defines clock providers and how clocks are managed in the kernel.

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

#[cfg(kernel_machine = "stm32f769i-disco")]
use crate::LockedArc;

/// The trait that all clock providers must implement
#[enum_dispatch::enum_dispatch]
pub trait ClockProviderTrait {
    /// Enable the specified clock
    fn enable_clock(&self, i: usize);
    /// Disable the specified clock
    fn disable_clock(&self, i: usize);
    /// Is the specified clock ready?
    fn clock_is_ready(&self, i: usize) -> bool;
    /// What is the frequency of the clock (if it is known)
    fn clock_frequency(&self, i: usize) -> Option<u64>;
    /// Get a `ClockRef` object from the provider
    fn get_ref(&self, i: usize) -> ClockRef;
}

/// An enumeration of all the types of gpio controllers
#[derive(Clone)]
#[enum_dispatch::enum_dispatch(ClockProviderTrait)]
pub enum ClockProvider {
    /// The main clock provider for the stm32f769
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769Provider(LockedArc<stm32f769::ClockTree>),
    /// The main pll for the stm32f769 processor
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32F769MainPll(stm32f769::PllMain),
    /// The second pll for the stm32f769 processor
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32F769SecondPll(stm32f769::PllTwo),
    /// The third pll for the stm32f769 processor
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32F769ThirdPll(stm32f769::PllThree),
    /// The dsi pll
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769DsiPll(crate::modules::video::mipi_dsi::stm32f769::Module),
    /// A fake clock provider
    Dummy(DummyClock),
}

/// A fixed frequency clock provider
#[derive(Clone)]
pub struct FixedClock {
    /// The frequency in hertz
    f: Option<u64>,
}

impl FixedClock {
    /// Construct a clock
    pub fn new(f: Option<u64>) -> Self {
        Self { f }
    }
}

impl ClockRefTrait for FixedClock {
    fn disable_clock(&self) {}

    fn enable_clock(&self) {}

    fn clock_is_ready(&self) -> bool {
        true
    }

    fn clock_frequency(&self) -> Option<u64> {
        self.f
    }
}

/// A fake clock provider
#[derive(Clone)]
pub struct DummyClock {}

impl Default for DummyClock {
    fn default() -> Self {
        Self::new()
    }
}

impl DummyClock {
    /// Construct a dummy clock
    pub fn new() -> Self {
        Self {}
    }
}

impl ClockProviderTrait for DummyClock {
    fn disable_clock(&self, _i: usize) {}

    fn enable_clock(&self, _i: usize) {}

    fn clock_is_ready(&self, _i: usize) -> bool {
        true
    }

    fn clock_frequency(&self, _i: usize) -> Option<u64> {
        None
    }

    fn get_ref(&self, _i: usize) -> ClockRef {
        panic!("Invalid clock");
    }
}

/// The trait for a single clock
#[enum_dispatch::enum_dispatch]
pub trait ClockRefTrait {
    /// Get the frequency of the clock, if known
    fn clock_frequency(&self) -> Option<u64>;
    /// Is the clock ready
    fn clock_is_ready(&self) -> bool;
    /// Enable the clock, if possible
    fn enable_clock(&self);
    /// Disable the clock, if possible
    fn disable_clock(&self);
}

/// A reference to a single clock
#[derive(Clone)]
#[enum_dispatch::enum_dispatch(ClockRefTrait)]
pub enum ClockRef {
    /// A fixed frequency clock
    Fixed(FixedClock),
    /// A regular reference directly to a clock provider
    Plain(ClockRefPlain),
    /// A clock from a mux
    Mux(ClockMux),
    /// The input clock divider for the the main, i2s, and sai pll
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769MainDivider(stm32f769::Divider1),
    /// A do nothing clock ref
    Dummy(DummyClock),
}

/// A clock reference to a single clock.
#[derive(Clone)]
pub struct ClockRefPlain {
    /// The provider of the clock
    pub clock_provider: ClockProvider,
    /// The index of the clock for the provider
    pub index: usize,
}

impl ClockRefTrait for ClockRefPlain {
    fn clock_frequency(&self) -> Option<u64> {
        self.clock_provider.clock_frequency(self.index)
    }

    fn clock_is_ready(&self) -> bool {
        self.clock_provider.clock_is_ready(self.index)
    }

    fn enable_clock(&self) {
        self.clock_provider.enable_clock(self.index);
    }

    fn disable_clock(&self) {
        self.clock_provider.disable_clock(self.index);
    }
}

impl ClockRefTrait for DummyClock {
    fn clock_frequency(&self) -> Option<u64> {
        None
    }

    fn clock_is_ready(&self) -> bool {
        true
    }

    fn enable_clock(&self) {}

    fn disable_clock(&self) {}
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
    /// The mux for the sysclk on the stm32f769
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769SysClkMux(stm32f769::MuxSysClk),
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

/// The trait for devices that contain one or more pll
#[enum_dispatch::enum_dispatch]
pub trait PllProviderTrait {
    /// Run a closure on the specified pll, returning a generic type (a Result might be beneficial).
    fn run_closure<T>(&self, i: u8, c: &dyn Fn(&mut Pll) -> T) -> Option<T>;
    /// Get a reference to the specified pll
    fn get_pll_reference(&self, i: u8) -> Option<Pll>;
    /// Get the specified clock mux
    fn get_clock_mux(&self, i: u32) -> Option<ClockMux>;
}

/// An enumeration of all the types of pll providers
#[derive(Clone)]
#[enum_dispatch::enum_dispatch(PllProviderTrait)]
pub enum PllProvider {
    /// A dummy for compilation
    DummyProvider(DummyPllProvider),
    /// The pll provider for the stm32f769 hardware
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(LockedArc<stm32f769::ClockTree>),
}

/// A dummy provider of pll objects. Simply panics.
#[derive(Clone)]
pub struct DummyPllProvider {}

impl PllProviderTrait for DummyPllProvider {
    fn run_closure<T>(&self, _i: u8, _c: &dyn Fn(&mut Pll) -> T) -> Option<T> {
        None
    }

    fn get_pll_reference(&self, _i: u8) -> Option<Pll> {
        None
    }

    fn get_clock_mux(&self, _i: u32) -> Option<ClockMux> {
        None
    }
}

/// The trait common to all pll providers
#[enum_dispatch::enum_dispatch]
pub trait PllTrait: ClockProviderTrait {
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
#[enum_dispatch::enum_dispatch(ClockProviderTrait, PllTrait)]
pub enum Pll {
    /// The main pll for the stm32f769
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769MainPll(stm32f769::PllMain),
    /// The second pll for the stm32f769
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769SecondPll(stm32f769::PllTwo),
    /// The third pll for the stm32f769
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769ThirdPll(stm32f769::PllThree),
    /// The dsi pll for the stm32f769
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769DsiPll(crate::modules::video::mipi_dsi::stm32f769::Module),
    /// A dummy pll provider
    Dummy(DummyClock),
}

impl PllTrait for DummyClock {
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
#[derive(Debug)]
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

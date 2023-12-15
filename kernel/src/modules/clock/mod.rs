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
}

#[derive(Clone)]
#[enum_dispatch::enum_dispatch(ClockProviderTrait)]
/// An enumeration of all the types of gpio controllers
pub enum ClockProvider {
    /// The reset provider for the stm32f769i-disco board.
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(LockedArc<crate::modules::reset::stm32f769::Module<'static>>),
    /// A fake clock provider
    Dummy(DummyClock),
}

/// A fake clock provider
#[derive(Clone)]
pub struct DummyClock {}

impl ClockProviderTrait for DummyClock {
    fn disable(&self, i: usize) {}

    fn enable(&self, i: usize) {}
}

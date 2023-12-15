//! For hardware that controls the reset lines of other peripherals

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

use crate::LockedArc;

/// The trait that all clock providers must implement
#[enum_dispatch::enum_dispatch]
pub trait ResetProviderTrait {
    /// Disable the specified reset
    fn enable(&self, i: usize);
    /// Enable the specified reset
    fn disable(&self, i: usize);
}

#[enum_dispatch::enum_dispatch(ResetProviderTrait)]
/// An enumeration of all the types of gpio controllers
pub enum ResetProvider {
    /// The reset provider for the stm32f769i-disco board.
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(LockedArc<stm32f769::Module<'static>>),
    /// A fake clock provider
    Dummy(DummyReset),
}

/// A fake clock provider
pub struct DummyReset {}

impl ResetProviderTrait for DummyReset {
    fn disable(&self, i: usize) {}
    fn enable(&self, i: usize) {}
}

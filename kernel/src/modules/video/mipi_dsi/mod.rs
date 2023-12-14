//! Code for mipi-dsi hardware

use crate::Locked;

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

/// The trait that all mipi dsi providers must implement
#[enum_dispatch::enum_dispatch]
pub trait MipiDsiTrait {
    /// Enable the hardware
    fn enable(&self);
    /// Disable the hardware
    fn disable(&self);
}

#[enum_dispatch::enum_dispatch(MipiDsiTrait)]
/// An enumeration of all the types of gpio controllers
pub enum MipiDsiProvider {
    /// The reset provider for the stm32f769i-disco board.
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(stm32f769::Module),
    /// A fake clock provider
    Dummy(DummyMipiCsi),
}

/// A fake clock provider
pub struct DummyMipiCsi {}

impl MipiDsiTrait for DummyMipiCsi {
    fn disable(&self) {}

    fn enable(&self) {}
}

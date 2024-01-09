//! Code for mipi-dsi hardware

use crate::Locked;

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

/// The configuration parameters for enabling dsi hardware
pub struct MipiDsiConfig {
    /// The speed of the link in bits per second
    pub link_speed: u64,
    /// The number of lanes in the link
    pub num_lanes: u8,
    /// The vcid of the display to display onto
    pub vcid: u8,
}

/// The trait that all mipi dsi providers must implement
#[enum_dispatch::enum_dispatch]
pub trait MipiDsiTrait {
    /// Enable the hardware
    fn enable(&self, config: &MipiDsiConfig, resolution: &super::ScreenResolution);
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

/// A DCS command that can be sent over mipi
pub enum DcsCommand<'a> {
    /// A short command
    Short(u8),
    /// A short command with a single parameter
    ShortParam(u8, u8),
    /// A long command with many parameters
    Long(&'a [u8]),
}

/// A fake clock provider
pub struct DummyMipiCsi {}

impl MipiDsiTrait for DummyMipiCsi {
    fn disable(&self) {}

    fn enable(&self, _config: &MipiDsiConfig, _resolution: &super::ScreenResolution) {}
}

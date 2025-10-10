//! Dummy code for various structs

#[derive(Clone, Default)]
/// Ethernet driver for the intel pro/1000 ethernet controller on pci
pub struct DummyPciDriver {}

impl DummyPciDriver {
    /// Build a new dummy driver
    pub fn new() -> Self {
        Self {}
    }
}

impl super::pci::PciFunctionDriverTrait for DummyPciDriver {}

//! X86 pci bus code

use crate::boot::x86::{IoPortRef, IOPORTS};

/// The x86 pci bus instance
pub struct PciBus {}

impl PciBus {
    /// Attempt to construct a pci bus
    pub fn new() -> Option<Self> {
        let pcia_address: IoPortRef<u32> = IOPORTS.get_port(0xcf8)?;
        let pcia_data: IoPortRef<u32> = IOPORTS.get_port(0xcfc)?;
        None
    }
}

impl super::PciBusTrait for PciBus {
    fn setup(&self) {}
}

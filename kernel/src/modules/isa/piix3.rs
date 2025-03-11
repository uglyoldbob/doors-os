//! Code for function 0 of the PIIX3 ISA bridge
//! <https://theretroweb.com/chip/documentation/82371sb-intelcorporation-62f8fda6daac0627455529.pdf>

#[doors_macros::enum_variant(IsaBus)]
/// The main struct for the isa bridge
pub struct IsaPiix3Bridge {}

impl super::IsaBusTrait for IsaPiix3Bridge {
    fn test(&self) -> bool {
        false
    }
}

/// The pci driver for the bridge
#[derive(Clone)]
pub struct IsaPiix3BridgeDriver {}

impl Default for IsaPiix3BridgeDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl IsaPiix3BridgeDriver {
    /// Construct a new self
    pub const fn new() -> Self {
        Self {}
    }
}

impl crate::modules::pci::PciFunctionDriverTrait for IsaPiix3BridgeDriver {
    async fn parse_bars(
        &mut self,
        _cs: &mut crate::modules::pci::PciConfigurationSpace,
        _bus: &crate::modules::pci::PciBus,
        _dev: &crate::modules::pci::PciDevice,
        _f: &crate::modules::pci::PciFunction,
        _config: &crate::modules::pci::ConfigurationSpaceEnum,
        _bars: [Option<crate::modules::pci::BarSpace>; 6],
    ) {
        todo!();
    }

    async fn register(
        &self,
        m: &mut alloc::collections::btree_map::BTreeMap<
            u32,
            crate::modules::PciFunctionDriver,
        >,
    ) {
        crate::VGA
            .print_str_async("Register intel piix3 ISA driver\r\n")
            .await;
        let vendor_combo = 0x7000_8086;
        m.entry(vendor_combo).or_insert_with(|| self.clone().into());
    }
}

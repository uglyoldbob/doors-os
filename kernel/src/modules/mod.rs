//! Kernel modules belong in this module. A lot of the enums will have a dummy provider so that the code will compile.

doors_macros::declare_enum!(PciFunctionDriver);

pub mod clock;
pub mod gpio;
pub mod isa;
pub mod memory;
pub mod network;
pub mod pci;
pub mod power;
pub mod reset;
pub mod rng;
pub mod serial;
pub mod timer;
pub mod video;

doors_macros2::enum_export_builder! {
    doors_macros2::enum_reexport!(PciFunctionDriver, network, isa);
}

/// The trait implemented for all devices
#[enum_dispatch::enum_dispatch]
pub trait DeviceTrait {}

/// A generic device in the kernel
#[enum_dispatch::enum_dispatch(DeviceTrait)]
pub enum Device {
    /// A single function of a pci device
    PciFunction(pci::PciFunction),
}

#[enum_dispatch::enum_dispatch]
/// The trait for all module implementations
pub trait ModuleTrait: Default {
    /// A test function to do something
    fn do_something(&self);
}

#[enum_dispatch::enum_dispatch(ModuleTrait)]
/// An enumeration of all the types of modules
pub enum Module {
    /// A test module
    Test(Test),
}

impl Default for Module {
    fn default() -> Self {
        Module::Test(Test::default())
    }
}

/// A test module
#[derive(Default)]
pub struct Test {}

impl ModuleTrait for Test {
    fn do_something(&self) {}
}

/// The trait that pci function drivers must implement
#[enum_dispatch::enum_dispatch]
pub trait PciFunctionDriverTrait: Clone {
    /// Register the driver in the given map, must check to see if the driver is already registered
    async fn register(&self, m: &mut alloc::collections::BTreeMap<u32, PciFunctionDriver>);

    /// Parse a bar register for the device
    async fn parse_bars(
        &mut self,
        cs: &mut PciConfigurationSpace,
        bus: &PciBus,
        dev: &PciDevice,
        f: &PciFunction,
        config: &ConfigurationSpaceEnum,
        bars: [Option<BarSpace>; 6],
    );
}

use pci::{
    BarSpace, ConfigurationSpaceEnum, PciBus, PciConfigurationSpace, PciDevice, PciFunction,
};

/// Represents a device driver for a pci function
#[doors_macros::fill_enum_with_variants_clonable(PciFunctionDriverTrait)]
pub enum PciFunctionDriver {}

doors_macros::todo_item!("Make this variable automated if possible");
/// Holds the pci drivers so that they can register with the `PCI_DRIVERS` variable
static PCI_CODE: &[PciFunctionDriver] = &[];

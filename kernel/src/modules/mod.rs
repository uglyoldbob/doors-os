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

use pci::{BarSpace, ConfigurationSpaceEnum, PciBus, PciConfigurationSpace, PciDevice, PciFunction, PciFunctionDriverTrait};

/// Represents a device driver for a pci function
#[enum_dispatch::enum_dispatch(PciFunctionDriverTrait)]
#[derive(Clone)]
pub enum PciFunctionDriver {
    /// A dummy driver so the enum isn't empty
    Dummy(pci::DummyPciFunctionDriver),
    /// Intel pro1000 ethernet driver
    IntelPro1000(crate::modules::network::intel::IntelPro1000),
    /// The PIIX3 isa bridge
    IntelPiix3IsaBridge(crate::modules::isa::piix3::IsaPiix3BridgeDriver),
}

doors_macros::todo_item!("Make this variable automated if possible");
/// Holds the pci drivers so that they can register with the `PCI_DRIVERS` variable
static PCI_CODE: &[PciFunctionDriver] = &[
    PciFunctionDriver::Dummy(pci::DummyPciFunctionDriver::new()),
    PciFunctionDriver::IntelPro1000(crate::modules::network::intel::IntelPro1000::new()),
    PciFunctionDriver::IntelPiix3IsaBridge(crate::modules::isa::piix3::IsaPiix3BridgeDriver::new()),
];

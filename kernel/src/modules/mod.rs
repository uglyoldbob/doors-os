//! Kernel modules belong in this module. A lot of the enums will have a dummy provider so that the code will compile.

pub mod clock;
pub mod dummy;
pub mod gpio;
pub mod input;
pub mod interrupt;
pub mod isa;
pub mod memory;
pub mod pci;
pub mod power;
pub mod reset;
pub mod rng;
pub mod serial;
pub mod timer;
pub mod video;

#[doors_macros::config_check_equals_attr(network, "true")]
pub mod network;

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

use pci::{
    BarSpace, ConfigurationSpaceEnum, PciBus, PciConfigurationSpace, PciDevice, PciFunction,
    PciFunctionDriverTrait,
};

/// Represents a device driver for a pci function
#[doors_macros::enum_module_filter]
#[derive(Clone)]
#[doors_macros::vec_builder]
#[enum_dispatch::enum_dispatch(PciFunctionDriverTrait)]
pub enum PciFunctionDriver {
    /// The piix3 isa bridge driver
    Piix3(isa::piix3::IsaPiix3BridgeDriver),
    /// The intel pro1000 pci driver
    #[doors_module = "intelpro1000"]
    IntelPro1000(network::intel::pro1000::IntelPro1000),
    /// A dummy pic function driver
    Dummy(pci::DummyPciFunctionDriver),
}

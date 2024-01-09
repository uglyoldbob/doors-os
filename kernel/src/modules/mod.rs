//! Kernel modules belong in this module. A lot of the enums will have a dummy provider so that the code will compile.

pub mod clock;
pub mod gpio;
pub mod memory;
pub mod power;
pub mod reset;
pub mod serial;
pub mod video;

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
pub struct Test {}

impl Default for Test {
    fn default() -> Self {
        Self {}
    }
}

impl ModuleTrait for Test {
    fn do_something(&self) {}
}

//! For gpio drivers

use crate::LockedArc;

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

#[enum_dispatch::enum_dispatch]
/// The trait for all gpio providers
pub trait GpioTrait {
    /// Get a specific gpio pin
    fn get_pin(&self, i: usize) -> Option<GpioPin>;
}

#[enum_dispatch::enum_dispatch(GpioTrait)]
/// An enumeration of all the types of gpio controllers
pub enum Gpio {
    /// The stm32f769 gpio module
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(LockedArc<stm32f769::Gpio>),
    /// The dummy implementation
    Dummy(DummyGpio),
}

/// A dummy gpio implementation
pub struct DummyGpio {}

impl GpioTrait for DummyGpio {
    fn get_pin(&self, i: usize) -> Option<GpioPin> {
        None
    }
}

impl GpioPinTrait for DummyGpio {
    fn set_output(&mut self) {}
    fn write_output(&mut self, _v: bool) {}
    fn set_alternate(&mut self, _mode: u8) {}
    fn set_speed(&mut self, _speed: u8) {}
}

#[enum_dispatch::enum_dispatch]
/// The trait for all gpio implementations
pub trait GpioPinTrait {
    /// Set a gpio as an output
    fn set_output(&mut self);
    /// Write a gpio value
    fn write_output(&mut self, v: bool);
    /// Set the alternate mode for a gpio pin
    fn set_alternate(&mut self, mode: u8);
    /// Set the output speed of the gpio pin
    fn set_speed(&mut self, speed: u8);
}

#[enum_dispatch::enum_dispatch(GpioPinTrait)]
/// An enumeration of all the types of gpio controllers
pub enum GpioPin {
    /// The stm32f769 gpio module
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(stm32f769::GpioPin),
    /// A placeholder dummy implementation of a gpio pin
    Dummy(DummyGpio),
}

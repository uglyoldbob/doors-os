//! For gpio drivers

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

#[enum_dispatch::enum_dispatch]
/// The trait for all gpio implementations
pub trait GpioTrait {
    /// Set a gpio as an output
    fn set_output(&mut self, i: usize);
    /// Write a gpio value
    fn write_output(&mut self, i: usize, v: bool);
    /// Get a specific gpio pin
    fn get_pin(&self, i: usize) -> Option<GpioPin>;
}

#[enum_dispatch::enum_dispatch(GpioTrait)]
/// An enumeration of all the types of gpio controllers
pub enum Gpio {
    /// The stm32f769 gpio module
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(stm32f769::Gpio<'static>),
    /// The dummy implementation
    Dummy(DummyGpio),
}

/// A dummy gpio implementation
pub struct DummyGpio {}

impl GpioTrait for DummyGpio {
    fn set_output(&mut self, i: usize) {}

    fn write_output(&mut self, i: usize, v: bool) {}

    fn get_pin(&self, i: usize) -> Option<GpioPin> {
        None
    }
}

impl GpioPinTrait for DummyGpio {
    fn set_output(&mut self) {}
    fn write_output(&mut self, v: bool) {}
}

#[enum_dispatch::enum_dispatch]
/// The trait for all gpio implementations
pub trait GpioPinTrait {
    /// Set a gpio as an output
    fn set_output(&mut self);
    /// Write a gpio value
    fn write_output(&mut self, v: bool);
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

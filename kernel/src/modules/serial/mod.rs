//! Serial port related code

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

use crate::LockedArc;

/// The standard trait for serial ports
#[enum_dispatch::enum_dispatch]
pub trait SerialTrait {
    /// Setup the serial port
    fn setup(&self, rate: u32) -> Result<(), ()>;
}

#[enum_dispatch::enum_dispatch(SerialTrait)]
/// An enumeration of all the types of serial controllers
pub enum Serial {
    /// The stm32f769 gpio module
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(LockedArc<stm32f769::Usart>),
    /// The dummy implementation
    Dummy(DummySerial),
}

/// A dummy serial port that does nothing
pub struct DummySerial {}

impl SerialTrait for DummySerial {
    fn setup(&self, rate: u32) -> Result<(), ()> {
        Err(())
    }
}

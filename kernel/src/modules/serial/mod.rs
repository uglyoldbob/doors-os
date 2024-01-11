//! Serial port related code

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

use crate::LockedArc;

/// The standard trait for serial ports
#[enum_dispatch::enum_dispatch]
pub trait SerialTrait {
    /// Setup the serial port
    fn setup(&self, rate: u32) -> Result<(), ()>;
    /// Transmit some data synchronously. This function returns once all data has been sent.
    fn sync_transmit(&self, data: &[u8]);
    /// Transmit some str data synchronously.
    fn sync_transmit_str(&self, data: &str);
}

/// An enumeration of all the types of serial controllers
#[enum_dispatch::enum_dispatch(SerialTrait)]
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

    fn sync_transmit(&self, data: &[u8]) {}

    fn sync_transmit_str(&self, data: &str) {}
}

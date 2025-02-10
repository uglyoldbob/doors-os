//! Serial port related code

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

#[cfg(kernel_machine = "pc64")]
pub mod x86;

use crate::LockedArc;

/// The standard trait for serial ports
#[enum_dispatch::enum_dispatch]
pub trait SerialTrait {
    /// Enable any required interrupts
    fn enable_interrupts(&self) -> Result<(), ()>;
    /// Setup the serial port
    fn setup(&self, rate: u32) -> Result<(), ()>;
    /// Transmit some data synchronously. Data may not be fully sent until flush is performed.
    fn sync_transmit(&self, data: &[u8]);
    /// Transmit some str data synchronously. Data may not be fully sent until flush is performed.
    fn sync_transmit_str(&self, data: &str);
    /// Flush all output data, synchronously
    fn sync_flush(&self);
    /// Transmit some data asynchronously. Data may not be fully sent until flush is performed.
    async fn transmit(&self, data: &[u8]);
    /// Transmit some str data asynchronously. Data may not be fully sent until flush is performed.
    async fn transmit_str(&self, data: &str);
    /// Flush the output data, asynchronously
    async fn flush(&self);
}

/// An enumeration of all the types of serial controllers
#[enum_dispatch::enum_dispatch(SerialTrait)]
pub enum Serial {
    /// The stm32f769 serial module
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(LockedArc<stm32f769::Usart>),
    /// X86 serial port
    #[cfg(kernel_machine = "pc64")]
    PcComPort(LockedArc<x86::X86SerialPort>),
    /// The dummy implementation
    Dummy(DummySerial),
}

impl LockedArc<Serial> {
    /// Create a text display
    pub fn make_text_display(&self) -> super::video::TextDisplay {
        let sd = super::video::VideoOverSerial::new(self.clone());
        super::video::TextDisplay::SerialDisplay(sd)
    }
}

/// A dummy serial port that does nothing
pub struct DummySerial {}

impl SerialTrait for DummySerial {
    fn setup(&self, _rate: u32) -> Result<(), ()> {
        Err(())
    }

    fn enable_interrupts(&self) -> Result<(), ()> {
        Err(())
    }

    fn sync_transmit(&self, _data: &[u8]) {}

    fn sync_transmit_str(&self, _data: &str) {}

    fn sync_flush(&self) {}

    async fn flush(&self) {}

    async fn transmit(&self, _data: &[u8]) {}

    async fn transmit_str(&self, _data: &str) {}
}

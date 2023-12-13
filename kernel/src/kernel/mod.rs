//! This is where the kernel structures are defined and where the code for interacting with them lives.

use crate::Locked;
use alloc::vec::Vec;
use lazy_static::lazy_static;

/// This is the main struct for interacting with the gpio system
pub struct GpioHandler {
    gpios: Vec<crate::modules::gpio::Gpio>,
}

impl GpioHandler {
    fn new() -> Self {
        Self { gpios: Vec::new() }
    }

    /// Add a gpio module to the system
    pub fn register_gpios(&mut self, g: crate::modules::gpio::Gpio) {
        self.gpios.push(g);
    }
}

lazy_static! {
    /// The entire list of io ports for an x86 machine
    pub static ref GPIO: Locked<GpioHandler> =
        Locked::new(GpioHandler::new());
}

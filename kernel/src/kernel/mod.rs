//! This is where the kernel structures are defined and where the code for interacting with them lives.

use crate::Locked;
use alloc::{sync::Arc, vec::Vec};
use lazy_static::lazy_static;

/// This is the main struct for interacting with the gpio system
pub struct GpioHandler {
    /// The individual gpio modules
    gpios: Vec<Arc<Locked<crate::modules::gpio::Gpio>>>,
}

impl GpioHandler {
    /// Create a new empty set of gpio modules
    fn new() -> Self {
        Self { gpios: Vec::new() }
    }

    /// Add a gpio module to the system
    pub fn register_gpios(&mut self, g: crate::modules::gpio::Gpio) {
        self.gpios.push(Arc::new(Locked::new(g)));
    }

    /// Get a gpio module
    pub fn module(&mut self, i: usize) -> Arc<Locked<crate::modules::gpio::Gpio>> {
        self.gpios[i].clone()
    }
}

/// Tracks all of the serial ports in the system
pub struct SerialHandler {
    /// The individual devices
    devs: Vec<Arc<Locked<crate::modules::serial::Serial>>>,
}

impl SerialHandler {
    /// Create a new empty set of serial modules
    fn new() -> Self {
        Self {
            devs: Vec::new(),
        }
    }

    /// Add a serial module to the system
    pub fn register_serial(&mut self, m: crate::modules::serial::Serial) {
        self.devs.push(Arc::new(Locked::new(m)));
    }

    /// Get a serial module
    pub fn module(&mut self, i: usize) -> Arc<Locked<crate::modules::serial::Serial>> {
        self.devs[i].clone()
    }
}

lazy_static! {
    /// The entire list of gpios for a system
    pub static ref GPIO: Locked<GpioHandler> =
        Locked::new(GpioHandler::new());
    /// The list of all serial ports for the system
    pub static ref SERIAL: Locked<SerialHandler> = 
        Locked::new(SerialHandler::new());
}

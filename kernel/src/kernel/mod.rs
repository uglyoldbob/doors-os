//! This is where the kernel structures are defined and where the code for interacting with them lives.

use crate::{Locked, LockedArc};
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
    devs: Vec<LockedArc<crate::modules::serial::Serial>>,
}

impl SerialHandler {
    /// Create a new empty set of serial modules
    fn new() -> Self {
        Self { devs: Vec::new() }
    }

    /// Add a serial module to the system
    pub fn register_serial(&mut self, m: crate::modules::serial::Serial) {
        self.devs.push(LockedArc::new(m));
    }

    /// Does the module index exist?
    pub fn exists(&self, i: usize) -> bool {
        i < self.devs.len()
    }

    /// Get a serial module
    pub fn module(&mut self, i: usize) -> LockedArc<crate::modules::serial::Serial> {
        self.devs[i].clone()
    }
}

/// Tracks all timers in the kernel
pub struct TimerHandler {
    /// The timer providers
    timerp: Vec<LockedArc<crate::modules::timer::Timer>>,
}

impl TimerHandler {
    /// Create a new empty set of serial modules
    fn new() -> Self {
        Self { timerp: Vec::new() }
    }

    /// Add a serial module to the system
    pub fn register_timer(&mut self, m: crate::modules::timer::Timer) {
        self.timerp.push(LockedArc::new(m));
    }

    /// Get a serial module
    pub fn module(&mut self, i: usize) -> LockedArc<crate::modules::timer::Timer> {
        self.timerp[i].clone()
    }
}

/// Tracks all displays in the kernel
pub struct DisplayHandler {
    /// The dsi displays
    displays: Vec<LockedArc<crate::modules::video::Display>>,
}

impl DisplayHandler {
    /// Create a new handler
    pub fn new() -> Self {
        Self {
            displays: Vec::new(),
        }
    }

    /// Register a display
    pub fn register_display(&mut self, d: crate::modules::video::Display) {
        self.displays.push(LockedArc::new(d));
    }

    /// Does the module index exist?
    pub fn exists(&self, i: usize) -> bool {
        i < self.displays.len()
    }

    /// Get a display module
    pub fn module(&mut self, i: usize) -> LockedArc<crate::modules::video::Display> {
        self.displays[i].clone()
    }
}

/// Tracks all rng devices in the kernel
pub struct RngHandler {
    /// The timer providers
    rng: Vec<LockedArc<crate::modules::rng::Rng>>,
}

impl RngHandler {
    /// Create a new empty set of serial modules
    fn new() -> Self {
        Self { rng: Vec::new() }
    }

    /// Add a serial module to the system
    pub fn register_rng(&mut self, m: crate::modules::rng::Rng) {
        self.rng.push(LockedArc::new(m));
    }

    /// Get a rng module
    pub fn module(&mut self, i: usize) -> LockedArc<crate::modules::rng::Rng> {
        self.rng[i].clone()
    }
}

lazy_static! {
    /// The entire list of gpios for the kernel
    pub static ref GPIO: Locked<GpioHandler> =
        Locked::new(GpioHandler::new());
    /// The list of all serial ports for the kernel
    pub static ref SERIAL: Locked<SerialHandler> =
        Locked::new(SerialHandler::new());
    /// The list of all timers for the kernel
    pub static ref TIMERS: Locked<TimerHandler> =
        Locked::new(TimerHandler::new());
    /// The list of the displays for the kernel
    pub static ref DISPLAYS : Locked<DisplayHandler> =
        Locked::new(DisplayHandler::new());
    /// The list of rng devices for the kernel
    pub static ref RNGS : Locked<RngHandler> =
        Locked::new(RngHandler::new());
}

/// This trait defines system specific elements
#[enum_dispatch::enum_dispatch]
pub trait SystemTrait {
    /// Enable interrupts for the system
    fn enable_interrupts(&self);
    /// System required init code
    fn init(&mut self);
    /// Allocate some virtual memory not corresponding to physical RAM
    fn allocate_nonram_memory(&mut self, size: usize, alignment: usize) -> Option<Box<[u8]>>;
}

/// This struct implements the SystemTrait
#[enum_dispatch::enum_dispatch(SystemTrait)]
pub enum System<'a> {
    #[cfg(kernel_machine = "pc64")]
    /// The x86 64 system code
    X86_64(crate::boot::x86::boot64::X86System<'a>),
}

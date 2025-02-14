//! This is where the kernel structures are defined and where the code for interacting with them lives.

use core::pin::Pin;

use crate::{AsyncLocked, AsyncLockedArc, Locked, LockedArc};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
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
    devs: Vec<AsyncLockedArc<crate::modules::serial::Serial>>,
}

impl SerialHandler {
    /// Create a new empty set of serial modules
    fn new() -> Self {
        Self { devs: Vec::new() }
    }

    /// Add a serial module to the system
    pub fn register_serial(&mut self, m: crate::modules::serial::Serial) {
        self.devs.push(AsyncLockedArc::new(m));
    }

    /// Does the module index exist?
    pub fn exists(&self, i: usize) -> bool {
        i < self.devs.len()
    }

    /// Get a serial module
    pub fn module(&mut self, i: usize) -> AsyncLockedArc<crate::modules::serial::Serial> {
        self.devs[i].clone()
    }

    /// Iterate over all serial ports
    pub fn iter(&mut self) -> core::slice::Iter<AsyncLockedArc<crate::modules::serial::Serial>> {
        self.devs.iter()
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

impl Default for DisplayHandler {
    fn default() -> Self {
        Self::new()
    }
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

    /// Does the module index exist?
    pub fn exists(&self, i: usize) -> bool {
        i < self.rng.len()
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
    pub static ref RNGS : AsyncLocked<RngHandler> =
        AsyncLocked::new(RngHandler::new());
}

/// This trait defines system specific elements
#[enum_dispatch::enum_dispatch]
pub trait SystemTrait {
    /// Enable interrupts
    fn enable_interrupts(&self);
    /// Disable interrupts
    fn disable_interrupts(&self);
    /// Register a serial port handler
    fn register_irq_handler<F: FnMut() -> () + Send + Sync + 'static>(&self, irq: u8, handler: F);
    /// Enable IRQ
    fn enable_irq(&self, irq: u8);
    /// Disable IRQ
    fn disable_irq(&self, irq: u8);
    /// System required init code
    fn init(&self);
    /// Code to idle the system
    fn idle(&self);
    /// Code to conditionally idle the system based on a closure
    fn idle_if(&self, f: impl FnMut() -> bool);
    /// Print debug stuff for acpi
    async fn acpi_debug(&self);
}

/// This struct implements the SystemTrait
#[derive(Clone)]
#[enum_dispatch::enum_dispatch(SystemTrait)]
pub enum System {
    #[cfg(kernel_machine = "pc64")]
    /// The x86 64 system code
    X86_64(LockedArc<Pin<Box<crate::boot::x86::boot64::X86System<'static>>>>),
}

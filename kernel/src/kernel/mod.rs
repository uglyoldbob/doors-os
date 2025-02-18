//! This is where the kernel structures are defined and where the code for interacting with them lives.

use core::pin::Pin;

use crate::{AsyncLocked, Locked, LockedArc};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use lazy_static::lazy_static;

/// A container of type T for a device that must be returned to its original container
pub struct OwnedDevice<T> {
    /// The module that is contained
    module: Option<T>,
    /// The returning code
    ret: Box<dyn Fn(T) + Send>,
}

impl<T> OwnedDevice<T> {
    /// Convert a device from one type of device to another, specifying another level of return functionality
    pub fn convert<U: 'static>(
        mut self,
        conversion: impl FnOnce(T) -> U,
        retf: impl Fn(U) + Send + 'static,
    ) -> OwnedDevice<U> {
        let m = self.module.take();
        OwnedDevice {
            module: m.map(conversion),
            ret: Box::new(retf),
        }
    }

    /// Build a self that does not get returned to anywhere, making it a free-range item
    pub fn free_range(t: T) -> Self {
        Self {
            module: Some(t),
            ret: Box::new(|_| ()),
        }
    }
}

impl<T> core::ops::Deref for OwnedDevice<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.module.as_ref().unwrap()
    }
}

impl<T> core::ops::DerefMut for OwnedDevice<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.module.as_mut().unwrap()
    }
}

impl<T> Drop for OwnedDevice<T> {
    fn drop(&mut self) {
        let m = self.module.take();
        if let Some(m) = m {
            (self.ret)(m);
        }
    }
}

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

/// The handout for code wanting to take a serial module from the collection
pub struct OwnedSerialModule {
    /// The module that is contained
    module: Option<crate::modules::serial::Serial>,
    /// The index for the serial module in the handler
    index: usize,
    /// The handler to return the module to when finished
    handler: &'static Locked<SerialHandler>,
}

impl core::ops::Deref for OwnedSerialModule {
    type Target = crate::modules::serial::Serial;
    fn deref(&self) -> &Self::Target {
        self.module.as_ref().unwrap()
    }
}

impl core::ops::DerefMut for OwnedSerialModule {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.module.as_mut().unwrap()
    }
}

impl Drop for OwnedSerialModule {
    fn drop(&mut self) {
        self.handler
            .return_module(self.index, self.module.take().unwrap());
    }
}

/// Tracks all of the serial ports in the system
pub struct SerialHandler {
    /// The individual devices
    devs: Vec<Option<crate::modules::serial::Serial>>,
}

impl SerialHandler {
    /// Create a new empty set of serial modules
    fn new() -> Self {
        Self { devs: Vec::new() }
    }

    /// Add a serial module to the system
    pub fn register_serial(&mut self, m: crate::modules::serial::Serial) {
        self.devs.push(Some(m));
    }

    /// Does the module index exist?
    pub fn exists(&self, i: usize) -> bool {
        i < self.devs.len() && self.devs[i].is_some()
    }

    /// Borrow a serial module
    pub fn borrow_module(&mut self, i: usize) -> &mut Option<crate::modules::serial::Serial> {
        &mut self.devs[i]
    }

    /// Iterate over all serial ports
    pub fn iter(&mut self) -> core::slice::Iter<Option<crate::modules::serial::Serial>> {
        self.devs.iter()
    }
}

impl Locked<SerialHandler> {
    /// Get a serial device
    pub fn take_device(
        &'static self,
        i: usize,
    ) -> Option<OwnedDevice<crate::modules::serial::Serial>> {
        let mut s = self.sync_lock();
        let m = if i < s.devs.len() {
            s.devs[i].take()
        } else {
            None
        };
        drop(s);
        m.map(|a| OwnedDevice {
            module: Some(a),
            ret: Box::new(move |t| {
                self.return_module(i, t);
            }),
        })
    }

    /// Return a serial module
    fn return_module(&self, i: usize, m: crate::modules::serial::Serial) {
        let mut s = self.sync_lock();
        if s.devs[i].is_none() {
            s.devs[i].replace(m);
        }
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
    /// Disable interrupts for the given closure
    fn disable_interrupts_for(&self, mut f: impl FnMut()) {
        self.disable_interrupts();
        f();
        self.enable_interrupts();
    }
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
    /// A dummy do nothing system
    NullSys(NullSystem),
}

impl Default for System {
    fn default() -> Self {
        System::NullSys(NullSystem {})
    }
}

/// A system that only will be used on startup
#[derive(Clone)]
pub struct NullSystem {}

impl NullSystem {
    /// Create a new system. This is a const fn so it can be done at compile time. Into functions are not constant yet.
    pub const fn new_sys() -> System {
        System::NullSys(Self {})
    }
}

impl SystemTrait for NullSystem {
    fn enable_interrupts(&self) {}
    fn disable_interrupts(&self) {}
    fn register_irq_handler<F: FnMut() -> () + Send + Sync + 'static>(&self, _irq: u8, _handler: F) {}
    fn enable_irq(&self, _irq: u8) {}
    fn disable_irq(&self, _irq: u8) {}
    fn init(&self) {}
    fn idle(&self) {}
    fn idle_if(&self, _f: impl FnMut() -> bool) {}
    async fn acpi_debug(&self) {}
}
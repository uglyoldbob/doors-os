//! Timer related code

use alloc::boxed::Box;

#[cfg(kernel_machine = "stm32f769i-disco")]
use crate::LockedArc;
use crate::{Arc, IrqGuarded, IrqGuardedInner, IrqGuardedUse};

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod x86;

/// The errors that can occur obtaining a timer
#[derive(Debug)]
pub enum TimerError {
    /// The timer desired is in use
    TimerIsAlreadyUsed,
}

/// The trait implemented by timer provider implementations
#[enum_dispatch::enum_dispatch]
pub trait TimerTrait {
    /// Return the inner timer
    fn return_timer_inner(&mut self, i: u8, t: TimerInstanceInner);
    /// Get an inner timer
    fn get_timer_inner(&mut self, i: u8) -> Result<TimerInstanceInner, TimerError>;
    /// Get a timer instance
    fn get_timer(&mut self, i: u8) -> Result<TimerInstance, TimerError> {
        let i = self.get_timer_inner(i)?;
        let j = i.into();
        Ok(j)
    }
}

/// The inner trait implemented by a single timer instance
#[enum_dispatch::enum_dispatch]
pub trait TimerInstanceInnerTrait {
    /// Delay a specified number of milliseconds
    fn delay_ms(&self, ms: u32);
    /// Delay a specified number of microseconds
    fn delay_us(&self, us: u32);
    /// Start or restart a oneshot timer
    fn start_oneshot(&mut self);
    /// Get the irq guard inner
    fn get_guard_inner(&self) -> IrqGuardedInner;
    /// Handle the hardware interrupt, and return which channel fired the interrupt
    fn hardware_interrupt(&self) -> u8;
}

/// An enumeration the types of timer instances
#[enum_dispatch::enum_dispatch(TimerInstanceInnerTrait)]
pub enum TimerInstanceInner {
    /// The pit timer instance for x86
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    X86PitTimer(x86::PitInner),
    /// A dummy timer inner instance
    DummyInner(DummyTimerInner),
}

/// An enumeration of all the types of timers
#[enum_dispatch::enum_dispatch(TimerTrait)]
pub enum Timer {
    /// The stm32f769 timer module
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(LockedArc<stm32f769::TimerGroup>),
    /// The pit timer for x86
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    X86Pit(x86::Pit),
    /// The dummy implementation
    Dummy(DummyTimer),
}

/// An instance of a timer channel
pub struct TimerInstance {
    inner: Arc<IrqGuarded<TimerInstanceInner>>,
    callback: Option<Arc<Box<dyn Fn(IrqGuardedUse<TimerInstanceInner>) + Send + Sync + 'static>>>,
}

impl TimerInstance {
    /// Get the inner instance as a reference
    pub fn sync_use(&self) -> IrqGuardedUse<'_, TimerInstanceInner> {
        self.inner.sync_access()
    }

    #[inline(never)]
    fn handle_interrupt(this: &IrqGuarded<TimerInstanceInner>, cb: &mut Option<Arc<Box<dyn Fn(IrqGuardedUse<TimerInstanceInner>) + Send + Sync + 'static>>>) {
        let s = this.interrupt_access();
        let _channel = s.hardware_interrupt();
        doors_macros::todo_item!("Do something with the indicated channel");
        if let Some(c) = cb {
            c(s);
        }
    }

    /// Register an interrupt handler
    pub fn register_handler<
        F: Fn(IrqGuardedUse<TimerInstanceInner>) -> () + Send + Sync + 'static,
    >(
        &mut self,
        f: F,
    ) {
        use crate::kernel::SystemTrait;
        self.callback.replace(Arc::new(Box::new(f)));
        let s2 = self.inner.clone();
        let mut cb = self.callback.clone();
        crate::SYSTEM
            .read()
            .register_irq_handler(self.inner.irq(), move || Self::handle_interrupt(&s2, &mut cb));
    }
}

impl From<TimerInstanceInner> for TimerInstance {
    fn from(value: TimerInstanceInner) -> Self {
        let com = value.get_guard_inner();
        Self {
            inner: Arc::new(IrqGuarded::new(value, &com)),
            callback: None,
        }
    }
}

/// A dummy implementation of a timer
pub struct DummyTimer {}

/// An inner implementation for a dummy timer
pub struct DummyTimerInner {}

impl TimerInstanceInnerTrait for DummyTimerInner {
    fn hardware_interrupt(&self) -> u8 {
        panic!();
    }

    fn delay_ms(&self, _ms: u32) {
        panic!();
    }

    fn delay_us(&self, _us: u32) {
        panic!();
    }

    fn start_oneshot(&mut self) {
        panic!();
    }

    fn get_guard_inner(&self) -> IrqGuardedInner {
        panic!();
    }
}

impl TimerTrait for DummyTimer {
    fn get_timer_inner(&mut self, _i: u8) -> Result<TimerInstanceInner, TimerError> {
        Err(TimerError::TimerIsAlreadyUsed)
    }

    fn return_timer_inner(&mut self, _i: u8, _t: TimerInstanceInner) {
        panic!();
    }
}

//! Timer related code

use core::marker::PhantomData;

use alloc::boxed::Box;

#[cfg(kernel_machine = "stm32f769i-disco")]
use crate::LockedArc;
use crate::{
    Arc, IrqGuarded, IrqGuardedInner, IrqGuardedUse, NotSafeForInterrupts, SafeForInterrupts,
};

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

#[doors_macros::module_builtin_attr(hpet, "true")]
pub mod hpet;

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod x86;

/// The errors that can occur obtaining a timer
#[derive(Debug)]
pub enum TimerError {
    /// The timer desired is in use
    TimerIsAlreadyUsed,
    /// Invalid timer number specified
    InvalidTimerIndex,
}

/// An iterator over timer channels
pub enum TimerIterator<'a> {
    /// A dummy iterator
    Dummy(DummyTimerIterator<'a>),
    /// hpet iterator
    Hpet(hpet::HpetTimerIterator<'a>),
    /// The pit iterator
    Pit(DummyTimerIterator<'a>),
}

impl<'a> Iterator for TimerIterator<'a> {
    type Item = &'a mut TimerInstanceInner;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Dummy(t) => t.next(),
            Self::Hpet(t) => t.next(),
            Self::Pit(t) => t.next(),
        }
    }
}

/// The trait implemented by timer provider implementations. A timer provider provides one or more timer instances,
#[enum_dispatch::enum_dispatch]
pub trait TimerTrait {
    /// Iterate over all timer channels
    fn iter_mut(&mut self) -> TimerIterator<'_>;
    /// Get an inner timer
    fn get_timer_inner(&mut self, i: u8) -> Result<TimerInstanceInner, TimerError>;
    /// Get a timer instance
    fn get_timer(&mut self, i: u8) -> Result<TimerInstance, TimerError> {
        let i = self.get_timer_inner(i)?;
        let j = i.into();
        Ok(j)
    }
}

/// Implemented by timers that can delay for arbitrary periods of time
#[async_trait::async_trait]
pub trait ArbitraryTimerTrait {
    /// Delay a specified number of milliseconds. This will be eventually deprecated and removed.
    fn delay_ms_sync(&self, ms: u32);
    /// Delay a specified number of microseconds. This will be eventually deprecated and removed.
    fn delay_us_sync(&self, us: u32);
    /// Asynchronously delay the specified number of milliseconds
    async fn delay_ms_async(&self, ms: u32);
}

/// Delay the specified number of milliseconds asynchronously.
pub async fn delay_ms_async(ms: u32) {
    let mut timers = crate::kernel::TIMERS.sync_lock();
    for t in timers.iter_mut() {
        let mut t = t.sync_lock();
        for tm in t.iter_mut() {
            if let Some(tmm) = tm.supports_arbitrary_timing() {
                tmm.delay_ms_async(ms).await;
                return;
            }
        }
    }
    panic!()
}

/// Delay a specified number of milliseconds. This will eventually be deprecated and removed
pub fn delay_ms_sync(ms: u32) {
    let mut timers = crate::kernel::TIMERS.sync_lock();
    for t in timers.iter_mut() {
        let mut t = t.sync_lock();
        for tm in t.iter_mut() {
            if let Some(tmm) = tm.supports_arbitrary_timing() {
                tmm.delay_ms_sync(ms);
                return;
            }
        }
    }
    panic!()
}

/// The inner trait implemented by a single timer instance
#[enum_dispatch::enum_dispatch]
pub trait TimerInstanceInnerTrait {
    /// Does the timer support arbitrary timing
    fn supports_arbitrary_timing(&self) -> Option<&dyn ArbitraryTimerTrait>;
    /// Start or restart a oneshot timer
    fn start_oneshot(&mut self);
    /// Get the irq guard inner, used to construct an individual timer channel
    fn get_guard_inner(&self) -> IrqGuardedInner;
    /// Handle the hardware interrupt, and return which channel fired the interrupt
    fn hardware_interrupt(&self) -> u8;
}

/// An enumeration the types of timer instances
#[doors_macros::enum_module_filter]
#[enum_dispatch::enum_dispatch(TimerInstanceInnerTrait)]
pub enum TimerInstanceInner {
    /// The pit timer instance for x86
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    X86PitTimer(x86::PitInner),
    /// A single channel for the hpet timer
    #[doors_module = "hpet"]
    HpetChannel(hpet::HpetChannel),
    /// A dummy timer inner instance
    DummyInner(DummyTimerInner),
}

/// An enumeration of all the types of timers
#[doors_macros::enum_module_filter]
#[enum_dispatch::enum_dispatch(TimerTrait)]
pub enum Timer {
    /// The stm32f769 timer module
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(LockedArc<stm32f769::TimerGroup>),
    /// The pit timer for x86
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    X86Pit(x86::Pit),
    /// The hpet timer (usually for x86 platforms)
    #[doors_module = "hpet"]
    Hpet(hpet::Hpet),
    /// The dummy implementation
    Dummy(DummyTimer),
}

/// The type for a callback function in the timer code
type TimerCallback = dyn Fn(IrqGuardedUse<TimerInstanceInner, SafeForInterrupts>)
    + crate::Interrupt
    + Send
    + Sync
    + 'static;

/// An instance of a timer channel
pub struct TimerInstance {
    /// The protected inner timer instance
    inner: Arc<IrqGuarded<TimerInstanceInner>>,
    /// The callback (will be moved to the [TimerInstanceInner] soon)
    callback: Option<Arc<Box<TimerCallback>>>,
}

impl TimerInstance {
    /// Get the inner instance as a reference
    pub fn sync_use(&self) -> IrqGuardedUse<'_, TimerInstanceInner, NotSafeForInterrupts> {
        self.inner.sync_access()
    }

    /// The interrupt handler for timers
    #[inline(never)]
    fn handle_interrupt(
        this: &IrqGuarded<TimerInstanceInner>,
        cb: &Option<Arc<Box<TimerCallback>>>,
    ) {
        let s = this.interrupt_access();
        let _channel = s.hardware_interrupt();
        doors_macros::todo_item!("Do something with the indicated channel");
        if let Some(c) = cb {
            c(s);
        }
    }

    /// Register an interrupt handler
    pub fn register_handler<
        F: Fn(IrqGuardedUse<TimerInstanceInner, SafeForInterrupts>)
            + crate::Interrupt
            + Send
            + Sync
            + 'static,
    >(
        &mut self,
        f: F,
    ) {
        use crate::kernel::SystemTrait;
        self.callback.replace(Arc::new(Box::new(f)));
        let s2 = self.inner.clone();
        let cb = self.callback.clone();
        crate::SYSTEM
            .read()
            .register_irq_handler(self.inner.irqs().next().unwrap(), move || {
                Self::handle_interrupt(&s2, &cb)
            });
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

/// An iterator over nothing
pub struct DummyTimerIterator<'a> {
    phantom: PhantomData<&'a usize>,
}

impl<'a> Iterator for DummyTimerIterator<'a> {
    type Item = &'a mut TimerInstanceInner;
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

/// A dummy implementation of a timer
pub struct DummyTimer {}

/// An inner implementation for a dummy timer
pub struct DummyTimerInner {}

impl Drop for DummyTimerInner {
    fn drop(&mut self) {}
}

impl TimerInstanceInnerTrait for DummyTimerInner {
    fn hardware_interrupt(&self) -> u8 {
        panic!();
    }

    fn supports_arbitrary_timing(&self) -> Option<&dyn ArbitraryTimerTrait> {
        None
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

    fn iter_mut(&mut self) -> TimerIterator<'_> {
        TimerIterator::Dummy(DummyTimerIterator {
            phantom: PhantomData,
        })
    }
}

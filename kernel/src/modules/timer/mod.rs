//! Timer related code

use core::marker::PhantomData;

use alloc::boxed::Box;

#[cfg(kernel_machine = "stm32f769i-disco")]
use crate::LockedArc;
use crate::{Arc, IrqGuardedInner, IrqGuardedUse, SafeForInterrupts};

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
pub enum TimerIterator {
    /// A dummy iterator
    Dummy(DummyTimerIterator),
    /// hpet iterator
    Hpet(hpet::HpetTimerIterator),
    /// The pit iterator
    Pit(DummyTimerIterator),
}

impl Iterator for TimerIterator {
    type Item = u8;
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
    fn iter(&self) -> TimerIterator;
    /// Get a timer channel
    fn get_timer(
        &mut self,
        i: u8,
        ms: u16,
        cb: Box<TimerCallback>,
    ) -> Result<TimerInstance, TimerError>;
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
    let mut timers = crate::kernel::TIMERS.lock().await;
    for t in timers.iter_mut() {
        let mut t = t.lock().await;
        for tm in t.iter() {
            let tmm = t.get_timer(tm, ms as u16, Box::new(|_| {}));
            if let Ok(tmm) = tmm {
                if let Some(at) = tmm.supports_arbitrary_timing() {
                    at.delay_ms_async(ms).await;
                    return;
                }
            }
        }
    }
    panic!()
}

/// Delay a specified number of milliseconds. This will eventually be deprecated and removed
pub fn delay_ms_sync(ms: u32) {
    let mut timers = crate::kernel::TIMERS.sync_lock();
    for (i, t) in timers.iter_mut().enumerate() {
        let mut t = t.sync_lock();
        for (j, tm) in t.iter().enumerate() {
            let tmm = t.get_timer(tm, ms as u16, Box::new(|_| {}));
            if let Ok(tmm) = tmm {
                if let Some(tmm) = tmm.supports_arbitrary_timing() {
                    tmm.delay_ms_sync(ms);
                    return;
                }
            }
        }
    }
    panic!()
}

/// The inner trait implemented by a single timer instance
#[enum_dispatch::enum_dispatch]
pub trait TimerInstanceTrait {
    /// Does the timer support arbitrary timing
    fn supports_arbitrary_timing(&self) -> Option<&dyn ArbitraryTimerTrait>;
    /// Start or restart a oneshot timer
    fn start_oneshot(&self);
}

/// An enumeration the types of timer instances
#[doors_macros::enum_module_filter]
#[derive(Clone)]
#[enum_dispatch::enum_dispatch(TimerInstanceTrait)]
pub enum TimerInstance {
    /// The pit timer instance for x86
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    X86PitTimer(Arc<x86::PitChannel>),
    /// A single channel for the hpet timer
    #[doors_module = "hpet"]
    HpetChannel(Arc<hpet::HpetChannel>),
    /// A dummy timer inner instance
    DummyInner(Arc<DummyTimerInner>),
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

/// The secondary type for a timer callback function
type TimerCallback2 = dyn Fn() + crate::Interrupt + Send + Sync + 'static;

/// The type for a callback function in the timer code
type TimerCallback = dyn Fn(TimerInstance) + crate::Interrupt + Send + Sync + 'static;

/// An iterator over nothing
pub struct DummyTimerIterator {}

impl Iterator for DummyTimerIterator {
    type Item = u8;
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

impl TimerInstanceTrait for Arc<DummyTimerInner> {
    fn supports_arbitrary_timing(&self) -> Option<&dyn ArbitraryTimerTrait> {
        None
    }

    fn start_oneshot(&self) {
        panic!();
    }
}

impl TimerTrait for DummyTimer {
    fn get_timer(
        &mut self,
        _i: u8,
        _ms: u16,
        _cb: Box<TimerCallback>,
    ) -> Result<TimerInstance, TimerError> {
        Err(TimerError::TimerIsAlreadyUsed)
    }

    fn iter(&self) -> TimerIterator {
        TimerIterator::Dummy(DummyTimerIterator {})
    }
}

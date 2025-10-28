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
    type Item = TimerInstance;
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
    fn get_timer_inner(&mut self, i: u8) -> Result<TimerInstance, TimerError>;
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
    for (i, t) in timers.iter_mut().enumerate() {
        crate::VGA.print_str(&alloc::format!(
            "QUERY Timer {} supports arbitrary timing\r\n",
            i
        ));
        let mut t = t.sync_lock();
        for (j, tm) in t.iter_mut().enumerate() {
            if let Some(tmm) = tm.supports_arbitrary_timing() {
                crate::VGA.print_str(&alloc::format!(
                    "Timer {},{} supports arbitrary timing\r\n",
                    i,
                    j
                ));
                tmm.delay_ms_sync(ms);
                return;
            } else {
                crate::VGA.print_str(&alloc::format!(
                    "Timer {},{} NOT SUPPORT arbitrary timing\r\n",
                    i,
                    j
                ));
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
    fn start_oneshot(&mut self);
    /// Set the interval for the timer in milliseconds
    fn set_interval(&mut self, interval: u16);
    /// Get the irq guard inner, used to construct an individual timer channel
    fn get_guard_inner(&self) -> IrqGuardedInner;
    /// Handle the hardware interrupt, and return which channel fired the interrupt
    fn hardware_interrupt(&self) -> u8;
    /// register an interrupt handler for this specific timer channel, returns true if successful
    fn register_handler(&mut self, f: Box<TimerCallback>) -> bool;
}

/// An enumeration the types of timer instances
#[doors_macros::enum_module_filter]
#[enum_dispatch::enum_dispatch(TimerInstanceTrait)]
pub enum TimerInstance {
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
type TimerCallback = dyn Fn(IrqGuardedUse<TimerInstance, SafeForInterrupts>)
    + crate::Interrupt
    + Send
    + Sync
    + 'static;

/// An iterator over nothing
pub struct DummyTimerIterator<'a> {
    phantom: PhantomData<&'a usize>,
}

impl<'a> Iterator for DummyTimerIterator<'a> {
    type Item = TimerInstance;
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

impl TimerInstanceTrait for DummyTimerInner {
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

    fn set_interval(&mut self, _interval: u16) {
        panic!();
    }

    fn register_handler(&mut self, _f: Box<TimerCallback>) -> bool {
        false
    }
}

impl TimerTrait for DummyTimer {
    fn get_timer_inner(&mut self, _i: u8) -> Result<TimerInstance, TimerError> {
        Err(TimerError::TimerIsAlreadyUsed)
    }

    fn iter_mut(&mut self) -> TimerIterator<'_> {
        TimerIterator::Dummy(DummyTimerIterator {
            phantom: PhantomData,
        })
    }
}

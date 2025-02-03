//! Timer related code

#[cfg(kernel_machine = "stm32f769i-disco")]
use crate::LockedArc;

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

/// The errors that can occur obtaining a timer
#[derive(Debug)]
pub enum TimerError {
    /// The timer desired is in use
    TimerIsAlreadyUsed,
}

/// The trait implemented by timer provider implementations
#[enum_dispatch::enum_dispatch]
pub trait TimerTrait {
    /// Get a timer
    fn get_timer(&mut self, i: u8) -> Result<TimerInstance, TimerError>;
}

/// The trait implemented by a single timer instance
#[enum_dispatch::enum_dispatch]
pub trait TimerInstanceTrait {
    /// Delay a specified number of milliseconds
    fn delay_ms(&self, ms: u32);
    /// Delay a specified number of microseconds
    fn delay_us(&self, us: u32);
}

/// An enumeration of all the types of timers
#[enum_dispatch::enum_dispatch(TimerTrait)]
pub enum Timer {
    /// The stm32f769 timer module
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(LockedArc<stm32f769::TimerGroup>),
    /// The dummy implementation
    Dummy(DummyTimer),
}

/// An enumeration the types of timer instances
#[enum_dispatch::enum_dispatch(TimerInstanceTrait)]
pub enum TimerInstance {
    /// A basic stm32f769 timer instance
    #[cfg(kernel_machine = "stm32f769i-disco")]
    BasicStm327f69Timer(LockedArc<stm32f769::Timer>),
    /// The dummy implementation
    Dummy(DummyTimer),
}

/// A dummy implementation of a timer
pub struct DummyTimer {}

impl TimerTrait for DummyTimer {
    fn get_timer(&mut self, _i: u8) -> Result<TimerInstance, TimerError> {
        Err(TimerError::TimerIsAlreadyUsed)
    }
}

impl TimerInstanceTrait for DummyTimer {
    fn delay_ms(&self, _ms: u32) {}

    fn delay_us(&self, _us: u32) {}
}

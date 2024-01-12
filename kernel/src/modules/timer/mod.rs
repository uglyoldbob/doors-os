//! Timer related code

use crate::LockedArc;

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

/// The trait implemented by timer implementations
#[enum_dispatch::enum_dispatch]
pub trait TimerTrait {
    /// Delay a specified number of milliseconds
    fn delay_ms(&self, ms: u32);
}

/// An enumeration of all the types of timers
#[enum_dispatch::enum_dispatch(TimerTrait)]
pub enum Timer {
    /// The stm32f769 timer module
    #[cfg(kernel_machine = "stm32f769i-disco")]
    Stm32f769(LockedArc<stm32f769::Timer>),
    /// The dummy implementation
    Dummy(DummyTimer),
}

/// A dummy implementation of a timer
pub struct DummyTimer {}

impl TimerTrait for DummyTimer {
    fn delay_ms(&self, _ms: u32) {}
}
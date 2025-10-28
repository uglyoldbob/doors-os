//! Code for x86 timers

use core::marker::PhantomData;

use alloc::boxed::Box;

use crate::{
    boot::IOPORTS, kernel::SystemTrait, Arc, IoPortRef, IoReadWrite, IrqGuardedInner, IrqNumbers,
};

doors_macros::todo_item!("Implement code for channel 2 of the pit, the speaker");

/// The inner structure for the [Pit]
pub struct PitInner {
    /// Channel 0
    chan0: IoPortRef<u8>,
    /// Channel 2
    _chan2: IoPortRef<u8>,
    /// command
    command: IoPortRef<u8>,
    /// The interrupt handler for channel0
    handler0: Option<Arc<Box<super::TimerCallback>>>,
}

impl PitInner {
    /// Attempt to construct a new self
    fn new() -> Option<Self> {
        let mut s = Self {
            chan0: IOPORTS.get_port(0x40)?,
            _chan2: IOPORTS.get_port(0x42)?,
            command: IOPORTS.get_port(0x43)?,
            handler0: None,
        };
        s.command.port_write(0);
        s.chan0.port_write(255);
        s.chan0.port_write(255);
        Some(s)
    }
}

impl Drop for PitInner {
    fn drop(&mut self) {}
}

impl super::TimerInstanceTrait for PitInner {
    fn hardware_interrupt(&self) -> u8 {
        0
    }

    fn register_handler(&mut self, f: Box<super::TimerCallback>) -> bool {
        let h = Arc::new(f);
        if self.handler0.is_none() {
            self.handler0.replace(h);
            true
        } else {
            false
        }
    }

    fn set_interval(&mut self, interval: u16) {
        let interval = interval as u64 * 1193182;
        let interval = interval / 1000;
        self.chan0.port_write((interval & 0xff) as u8);
        self.chan0.port_write(((interval >> 8) & 0xff) as u8);
    }

    fn supports_arbitrary_timing(&self) -> Option<&dyn super::ArbitraryTimerTrait> {
        None
    }

    fn get_guard_inner(&self) -> IrqGuardedInner {
        IrqGuardedInner::new(IrqNumbers::Only1(0), false, true, |_| {}, |_| {})
    }

    fn start_oneshot(&mut self) {
        let v = 65535u16;
        let v = v.to_le_bytes();

        self.command.port_write(8);
        self.chan0.port_write(v[0]);
        self.chan0.port_write(v[1]);
    }
}

/// The programmable interval timer for x86 hardware
pub struct Pit {
    /// protected data
    i: Option<PitInner>,
}

impl Pit {
    /// Disable the pit
    pub fn disable(&mut self) {
        crate::VGA.print_str("DISABLING PIT IRQ\r\n");
        unsafe { crate::SYSTEM.read().unregister_irq_handler(0) };
    }
}

impl Drop for Pit {
    fn drop(&mut self) {
        crate::VGA.print_str("DROPPING PIT\r\n");
    }
}

impl Default for Pit {
    fn default() -> Self {
        Self {
            i: Some(PitInner::new().unwrap()),
        }
    }
}

impl super::TimerTrait for Pit {
    fn get_timer_inner(&mut self, i: u8) -> Result<super::TimerInstance, super::TimerError> {
        assert_eq!(i, 0);
        if let Some(t) = self.i.take() {
            Ok(t.into())
        } else {
            Err(super::TimerError::TimerIsAlreadyUsed)
        }
    }

    fn iter_mut(&mut self) -> super::TimerIterator<'_> {
        super::TimerIterator::Pit(super::DummyTimerIterator {
            phantom: PhantomData,
        })
    }
}

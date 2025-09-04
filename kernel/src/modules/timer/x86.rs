//! Code for x86 timers

use crate::{boot::IOPORTS, IoPortRef, IoReadWrite, IrqGuardedInner, IrqNumbers};

doors_macros::todo_item!("Implement code for channel 2 of the pit, the speaker");

/// The inner structure for the [Pit]
pub struct PitInner {
    /// Channel 0
    chan0: IoPortRef<u8>,
    /// Channel 2
    _chan2: IoPortRef<u8>,
    /// command
    command: IoPortRef<u8>,
}

impl PitInner {
    /// Attempt to construct a new self
    fn new() -> Option<Self> {
        let mut s = Self {
            chan0: IOPORTS.get_port(0x40)?,
            _chan2: IOPORTS.get_port(0x42)?,
            command: IOPORTS.get_port(0x43)?,
        };
        s.command.port_write(0);
        s.chan0.port_write(255);
        s.chan0.port_write(255);
        Some(s)
    }
}

impl super::TimerInstanceInnerTrait for PitInner {
    fn hardware_interrupt(&self) -> u8 {
        0
    }

    fn delay_ms(&self, _ms: u32) {
        doors_macros::todo!();
    }

    fn get_guard_inner(&self) -> IrqGuardedInner {
        IrqGuardedInner::new(IrqNumbers::Only1(0), false, true, |_| {}, |_| {})
    }

    fn delay_us(&self, _us: u32) {
        doors_macros::todo!();
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

impl Default for Pit {
    fn default() -> Self {
        Self {
            i: Some(PitInner::new().unwrap()),
        }
    }
}

impl super::TimerTrait for Pit {
    fn get_timer_inner(&mut self, i: u8) -> Result<super::TimerInstanceInner, super::TimerError> {
        assert_eq!(i, 0);
        if let Some(t) = self.i.take() {
            Ok(t.into())
        } else {
            Err(super::TimerError::TimerIsAlreadyUsed)
        }
    }

    fn return_timer_inner(&mut self, i: u8, t: super::TimerInstanceInner) {
        assert_eq!(i, 0);
        if let super::TimerInstanceInner::X86PitTimer(a) = t {
            self.i.replace(a);
        }
    }
}

//! Code for x86 timers

use alloc::boxed::Box;

use crate::{
    boot::IOPORTS,
    kernel::{SoftwareInterruptTrait, SystemTrait},
    Arc, IoPortRef, IoReadWrite, IrqGuarded, IrqGuardedInner, IrqNumbers,
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
    handler0: super::TimerCallback2WithUsage,
    /// The software interrupt handler
    sint: Option<crate::kernel::SoftwareInterrupt>,
}

/// a pit channel
pub struct PitChannel {
    inner: Arc<IrqGuarded<PitInner>>,
}

impl PitInner {
    /// Attempt to construct a new self
    fn new() -> Option<Self> {
        let mut s = Self {
            chan0: IOPORTS.get_port(0x40)?,
            _chan2: IOPORTS.get_port(0x42)?,
            command: IOPORTS.get_port(0x43)?,
            handler0: super::TimerCallback2WithUsage::None,
            sint: None,
        };
        s.sint = crate::SYSTEM.read().get_software_interrupt(|| loop {});
        s.command.port_write(0);
        s.chan0.port_write(255);
        s.chan0.port_write(255);
        Some(s)
    }

    /// set the interval in milliseconds
    fn set_interval(&mut self, interval: u16) {
        let interval = interval as u64 * 1193182;
        let interval = interval / 1000;
        self.chan0.port_write((interval & 0xff) as u8);
        self.chan0.port_write(((interval >> 8) & 0xff) as u8);
    }
}

impl Drop for PitInner {
    fn drop(&mut self) {}
}

impl super::TimerInstanceTrait for Arc<PitChannel> {
    fn supports_arbitrary_timing(&self) -> Option<&dyn super::ArbitraryTimerTrait> {
        None
    }

    fn start_oneshot(&self) {
        let v = 65535u16;
        let v = v.to_le_bytes();
        let mut this = self.inner.sync_access();
        this.command.port_write(8);
        this.chan0.port_write(v[0]);
        this.chan0.port_write(v[1]);
    }

    fn manually_trigger(&self) {
        if let Some(s) = &self.inner.sync_access().sint {
            s.call();
        }
    }
}

/// The programmable interval timer for x86 hardware
pub struct Pit {
    /// protected data
    i: Option<Arc<IrqGuarded<PitInner>>>,
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
        let p = PitInner::new().unwrap();
        let com = IrqGuardedInner::new(IrqNumbers::Only1(0), false, true, |_| {}, |_| {});
        Self {
            i: Some(Arc::new(IrqGuarded::new(p, &com))),
        }
    }
}

impl super::TimerTrait for Pit {
    fn get_timer(
        &mut self,
        i: u8,
        ms: u16,
        cb: super::TimerCallbackWithUsage,
    ) -> Result<super::TimerInstance, super::TimerError> {
        assert_eq!(i, 0);
        if let Some(t) = self.i.take() {
            let h = PitChannel { inner: t.clone() };
            let mut t2 = t.sync_access();
            t2.set_interval(ms);
            let h2 = Arc::new(h);
            let h3: super::TimerInstance = h2.into();
            let h4 = h3.downgrade();
            match cb {
                super::TimerCallbackWithUsage::Single(a) => {
                    if let Some(a) = a {
                        t2.handler0 = super::TimerCallback2WithUsage::Single(Some(Arc::new(
                            Box::new(move || a(h4.clone())),
                        )));
                    }
                }
                super::TimerCallbackWithUsage::Multiple(a) => {
                    t2.handler0 =
                        super::TimerCallback2WithUsage::Multiple(Arc::new(Box::new(move || {
                            a(h4.clone())
                        })));
                }
                super::TimerCallbackWithUsage::None => {}
            }
            Ok(h3)
        } else {
            Err(super::TimerError::TimerIsAlreadyUsed)
        }
    }

    fn iter(&self) -> super::TimerIterator {
        super::TimerIterator::Pit(super::DummyTimerIterator {})
    }
}

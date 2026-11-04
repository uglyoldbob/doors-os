//! Timer modules for the stm32f769

use alloc::boxed::Box;

use crate::kernel::SystemTrait;
use crate::modules::clock::ClockRefTrait;
use crate::{Arc, LockedArc};

/// The number of capture/compare channels supported by the advanced control timers
const CHANNELS: usize = 6;

/// The register set for an advanced control timer (tim1/tim8)
struct Registers {
    regs: [u32; 64],
}

/// An iterator over the capture compare channels of the timer group
pub struct TimerIterator {
    /// The next channel to return
    cur: u8,
    /// The maximum channel to return
    max: u8,
}

impl Iterator for TimerIterator {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        if self.cur < self.max {
            let r = self.cur;
            self.cur += 1;
            Some(r)
        } else {
            None
        }
    }
}

/// The basic timer module. This covers functionality of timer 1 and 8
pub struct TimerGroup {
    /// The registers
    regs: &'static mut Registers,
    /// Indicates which of the individual clock modules exist.
    clocks_used: u8,
    /// Indicates if the prescaler has already been programmed
    configured: bool,
    /// Indicates if the nvic interrupt for this group has already been registered
    irq_registered: bool,
    /// The interrupt number for the capture compare interrupt of this timer
    irq: u8,
    /// The input clock to the timer.
    clock: crate::modules::clock::ClockRef,
    /// The callbacks for each of the channels
    callbacks: [super::TimerCallback2WithUsage; CHANNELS],
    /// Weak references to the instances handed out for each channel
    instances: [Option<super::WeakTimerInstance>; CHANNELS],
}

impl TimerGroup {
    /// Build a new timer
    pub unsafe fn new(clock: crate::modules::clock::ClockRef, addr: u32) -> Self {
        // The capture compare interrupt for tim1 is irq 27 and tim8 is irq 46
        let irq = if addr == 0x4001_0400 { 46 } else { 27 };
        Self {
            regs: &mut *(addr as *mut Registers),
            clocks_used: 0,
            configured: false,
            irq_registered: false,
            irq,
            clock,
            callbacks: core::array::from_fn(|_| super::TimerCallback2WithUsage::None),
            instances: core::array::from_fn(|_| None),
        }
    }

    /// The bit in the status/interrupt registers for a given channel
    fn cc_bit(i: u8) -> u32 {
        match i {
            0..=3 => 1 + i as u32,
            4..=5 => 12 + i as u32,
            _ => panic!("Invalid timer"),
        }
    }

    /// Program the prescaler so that the given period fits in the 16 bit counter.
    ///
    /// The prescaler is shared by every channel of the group, so it is only raised, and only
    /// when the value that is currently programmed cannot represent the requested period.
    fn program_prescaler(&mut self, freq: u64, ms: u16) {
        // Start with a tick that is close to 1 mhz and double the prescaler until the requested
        // period fits in the 16 bit counter.
        let mut prescaler = if self.configured {
            (self.prescaler() + 1) as u64
        } else {
            (freq / 1_000_000).max(1)
        };
        while prescaler < 0x10000 && (freq / prescaler) * (ms as u64) / 1000 > 65_535 {
            prescaler *= 2;
        }
        if prescaler > 0x10000 {
            prescaler = 0x10000;
        }
        if self.configured && (self.prescaler() + 1) as u64 == prescaler {
            return;
        }
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[10], (prescaler - 1) as u32) };
        self.update();
        self.configured = true;
    }

    /// Get the number of counter ticks that correspond to the specified number of microseconds
    fn micro_counts(&self, us: u64) -> u64 {
        let freq = self.clock.clock_frequency().unwrap_or(16_000_000);
        let prescaler = (self.prescaler() + 1) as u64;
        (freq / prescaler) * us / 1_000_000
    }

    /// Release the channel so it can be handed out again
    fn release_channel(&mut self, i: u8) {
        self.clocks_used &= !(1u8 << i);
    }

    /// Returns the prescaler for the timer
    fn prescaler(&self) -> u32 {
        unsafe { core::ptr::read_volatile(&self.regs.regs[10]) }
    }

    /// Returns the count of the timer
    fn counter(&self) -> u32 {
        let v = unsafe { core::ptr::read_volatile(&self.regs.regs[9]) };
        v & 0xFFFF
    }

    /// Start the timer if it is not already running
    fn start_timer(&mut self) {
        let c = unsafe { core::ptr::read_volatile(&self.regs.regs[0]) };
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[0], c | 1) };
    }

    /// Stop the timer
    fn stop_timer(&mut self) {
        let c = unsafe { core::ptr::read_volatile(&self.regs.regs[0]) };
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[0], c & !1) };
    }

    /// Generate an update event
    fn update(&mut self) {
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[0x14 / 4], 1) };
    }

    /// Generate a compare event for a channel
    fn generate_compare(&mut self, i: u8) {
        let v = unsafe { core::ptr::read_volatile(&self.regs.regs[0x14 / 4]) };
        unsafe {
            core::ptr::write_volatile(&mut self.regs.regs[0x14 / 4], v | (1 << (2 + i)));
        }
    }

    /// Enable or disable the capture compare interrupt for a channel
    fn set_ccie(&mut self, i: u8, on: bool) {
        let bit = Self::cc_bit(i);
        let v = unsafe { core::ptr::read_volatile(&self.regs.regs[0x0c / 4]) };
        let nv = if on { v | (1 << bit) } else { v & !(1 << bit) };
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[0x0c / 4], nv) };
    }

    /// Get if the capture compare interrupt is enabled for a channel
    fn get_ccie(&self, i: u8) -> bool {
        let bit = Self::cc_bit(i);
        let v = unsafe { core::ptr::read_volatile(&self.regs.regs[0x0c / 4]) };
        (v & (1 << bit)) != 0
    }

    /// Clear the compare flag
    fn clear_compare(&mut self, i: u8) {
        let bit = Self::cc_bit(i);
        let v = unsafe { core::ptr::read_volatile(&self.regs.regs[4]) };
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[4], v & !(1 << bit)) };
    }

    /// get the compare flag
    fn get_compare(&self, i: u8) -> bool {
        let bit = Self::cc_bit(i);
        let v = unsafe { core::ptr::read_volatile(&self.regs.regs[4]) };
        (v & (1 << bit)) != 0
    }

    /// Sets the ccr for the specified timer
    fn set_ccr(&mut self, i: u8, val: u16) {
        let reg = match i {
            0..=3 => 13 + i as usize,
            4..=5 => 18 + i as usize,
            _ => panic!("Invalid timer"),
        };
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[reg], val as u32) };
    }

    /// The interrupt handler for the capture compare interrupt of this timer group
    pub fn handle_interrupt(s: &LockedArc<TimerGroup>) {
        let mut g = s.sync_lock();
        let sr = unsafe { core::ptr::read_volatile(&g.regs.regs[4]) };
        for i in 0..CHANNELS as u8 {
            let bit = 1u32 << Self::cc_bit(i);
            if (sr & bit) == 0 {
                continue;
            }
            // Only consume the event if the channel actually has its interrupt enabled.
            // Delays poll the flag without enabling the interrupt and must not have their
            // flag stolen by this handler.
            if !g.get_ccie(i) {
                continue;
            }
            g.clear_compare(i);
            let h = g.callbacks[i as usize].take();
            let inst = g.instances[i as usize].clone();
            if h.is_some() {
                drop(g);
                if let (Some(h), Some(_inst)) = (h, inst) {
                    h();
                }
                g = s.sync_lock();
            }
        }
    }
}

impl super::TimerTrait for LockedArc<TimerGroup> {
    fn iter(&self) -> super::TimerIterator {
        super::TimerIterator::Stm32f769(TimerIterator { cur: 0, max: 4 })
    }

    fn get_timer(
        &mut self,
        i: u8,
        ms: u16,
        cb: super::TimerCallbackWithUsage,
    ) -> Result<super::TimerInstance, super::TimerError> {
        let mut s = self.sync_lock();
        if i as usize >= CHANNELS {
            return Err(super::TimerError::InvalidTimerIndex);
        }
        let check = 1u8 << i;
        if (s.clocks_used & check) != 0 {
            return Err(super::TimerError::TimerIsAlreadyUsed);
        }
        s.clocks_used |= check;
        s.clock.enable_clock();
        let freq = s.clock.clock_frequency().unwrap_or(16_000_000);
        s.program_prescaler(freq, ms);
        let timer = Arc::new(Timer {
            timer: self.clone(),
            index: i,
            ms,
        });
        let instance: super::TimerInstance = timer.clone().into();
        let weak = instance.downgrade();
        s.instances[i as usize] = Some(weak.clone());
        s.callbacks[i as usize] = match cb {
            super::TimerCallbackWithUsage::Single(a) => {
                if let Some(a) = a {
                    super::TimerCallback2WithUsage::Single(Some(Arc::new(Box::new(move || {
                        a(weak.clone())
                    }))))
                } else {
                    super::TimerCallback2WithUsage::None
                }
            }
            super::TimerCallbackWithUsage::Multiple(a) => {
                super::TimerCallback2WithUsage::Multiple(Arc::new(Box::new(move || {
                    a(weak.clone())
                })))
            }
            super::TimerCallbackWithUsage::None => super::TimerCallback2WithUsage::None,
        };
        let irq_registered = s.irq_registered;
        let irq = s.irq;
        drop(s);
        if !irq_registered {
            let mut s2 = self.sync_lock();
            s2.irq_registered = true;
            drop(s2);
            let group = self.clone();
            let sys = crate::SYSTEM.read();
            sys.register_irq_handler(irq, move || TimerGroup::handle_interrupt(&group));
            sys.enable_irq(irq);
        }
        Ok(instance)
    }
}

/// An individual timer of the basic timergroup.
pub struct Timer {
    /// Reference to the hardware
    timer: LockedArc<TimerGroup>,
    /// Which timer in specific this timer is.
    index: u8,
    /// The period in milliseconds this instance was requested for
    ms: u16,
}

impl Drop for Timer {
    fn drop(&mut self) {
        let mut t = self.timer.sync_lock();
        t.set_ccie(self.index, false);
        t.release_channel(self.index);
        t.callbacks[self.index as usize] = super::TimerCallback2WithUsage::None;
        t.instances[self.index as usize] = None;
    }
}

impl Arc<Timer> {
    /// Wait for the timer to count the given number of ticks, releasing the hardware lock while
    /// waiting so that the interrupt handler for this group can still make progress.
    fn delay_counts(&self, mut counts_required: u64) {
        loop {
            let chunk = counts_required.min(65_000);
            counts_required -= chunk;
            {
                let mut t = self.timer.sync_lock();
                t.clock.enable_clock();
                let ccr = (t.counter() as u64 + chunk) & 0xFFFF;
                t.set_ccr(self.index, ccr as u16);
                t.clear_compare(self.index);
                t.start_timer();
            }
            loop {
                let done = {
                    let t = self.timer.sync_lock();
                    t.get_compare(self.index)
                };
                if done {
                    break;
                }
                core::hint::spin_loop();
            }
            {
                let mut t = self.timer.sync_lock();
                if counts_required == 0 {
                    t.stop_timer();
                }
            }
            if counts_required == 0 {
                break;
            }
        }
    }

    /// Program the hardware to fire the compare flag for this channel after the given counts,
    /// without enabling the interrupt. Returns nothing, the flag is polled.
    fn arm_poll(&self, counts: u64) {
        let mut t = self.timer.sync_lock();
        t.clock.enable_clock();
        let ccr = (t.counter() as u64 + counts) & 0xFFFF;
        t.set_ccr(self.index, ccr as u16);
        t.clear_compare(self.index);
        t.start_timer();
    }

    /// Check and clean up a poll armed delay
    fn poll_done(&self) -> bool {
        let mut t = self.timer.sync_lock();
        let done = t.get_compare(self.index);
        if done {
            t.stop_timer();
        }
        done
    }
}

#[async_trait::async_trait]
impl super::ArbitraryTimerTrait for Arc<Timer> {
    fn delay_ms_sync(&self, ms: u32) {
        let counts = {
            let t = self.timer.sync_lock();
            t.micro_counts(ms as u64 * 1000)
        };
        self.delay_counts(counts);
    }

    fn delay_us_sync(&self, us: u32) {
        let counts = {
            let t = self.timer.sync_lock();
            t.micro_counts(us as u64)
        };
        self.delay_counts(counts);
    }

    async fn delay_ms_async(&self, ms: u32) {
        let counts = {
            let t = self.timer.sync_lock();
            t.micro_counts(ms as u64 * 1000)
        };
        let mut remaining = counts;
        loop {
            let chunk = remaining.min(65_000);
            remaining -= chunk;
            self.arm_poll(chunk);
            loop {
                if self.poll_done() {
                    break;
                }
                crate::executor::AsyncTask::yield_now().await;
            }
            if remaining == 0 {
                break;
            }
        }
    }
}

impl super::TimerInstanceTrait for Arc<Timer> {
    fn supports_arbitrary_timing(&self) -> Option<&dyn super::ArbitraryTimerTrait> {
        Some(self)
    }

    fn start_oneshot(&self) {
        let mut t = self.timer.sync_lock();
        t.clock.enable_clock();
        let freq = t.clock.clock_frequency().unwrap_or(16_000_000);
        t.program_prescaler(freq, self.ms);
        // The prescaler is shared by the whole group and can be raised by other users of the
        // group, so the compare value is always recomputed from the current tick rate.
        let mut counts = t.micro_counts(self.ms as u64 * 1000);
        if counts > 65_535 {
            counts = 65_535;
        }
        if counts == 0 {
            counts = 1;
        }
        let ccr = (t.counter() as u64 + counts) & 0xFFFF;
        t.set_ccr(self.index, ccr as u16);
        t.clear_compare(self.index);
        t.set_ccie(self.index, true);
        t.start_timer();
    }

    fn manually_trigger(&self) {
        let mut t = self.timer.sync_lock();
        t.generate_compare(self.index);
    }
}

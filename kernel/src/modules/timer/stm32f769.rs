//! Timer modules for the stm32f769

use crate::{modules::clock::ClockRefTrait, LockedArc};

struct Registers {
    regs: [u32; 16],
}

/// The basic timer module. This covers functionality of timer 1 and 8
pub struct TimerGroup {
    /// The registers
    regs: &'static mut Registers,
    /// Indicates which of the individual clock modules exist.
    clocks_used: u8,
    /// Indicates the use count for the timergroup
    usage: u8,
    /// The input clock to the timer.
    clock: crate::modules::clock::ClockRef,
}

impl TimerGroup {
    /// Build a new timer
    pub unsafe fn new(clock: crate::modules::clock::ClockRef, addr: u32) -> Self {
        Self {
            regs: &mut *(addr as *mut Registers),
            clocks_used: 0,
            usage: 0,
            clock,
        }
    }

    /// Declare that the timer is being used, return ok if it can be adjusted.
    fn try_adjust(&mut self, prescaler: u32) -> Result<(), ()> {
        if self.usage == 0 {
            if prescaler > 0x10000 {
                return Err(());
            }
            unsafe { core::ptr::write_volatile(&mut self.regs.regs[10], prescaler - 1) };
            self.update();
            self.usage += 1;
            Ok(())
        } else {
            self.usage += 1;
            Err(())
        }
    }

    /// Declare the timer is being less used
    fn unadjust(&mut self) {
        self.usage -= 1;
    }

    /// Returns the prescaler for the timer
    fn prescaler(&self) -> u32 {
        unsafe { core::ptr::read_volatile(&self.regs.regs[10]) }
    }

    /// Returns the status register
    fn status(&self) -> u32 {
        unsafe { core::ptr::read_volatile(&self.regs.regs[4]) }
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

    /// Clear the compare flag
    fn clear_compare(&mut self, i: u8) {
        let bit = match i {
            0..=3 => 1 + i,
            4..=5 => 12 + i,
            _ => panic!("Invalid timer"),
        };
        let v = unsafe { core::ptr::read_volatile(&self.regs.regs[4]) };
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[4], v & !(1 << bit)) };
    }

    /// get the compare flag
    fn get_compare(&self, i: u8) -> bool {
        let bit = match i {
            0..=3 => 1 + i,
            4..=5 => 12 + i,
            _ => panic!("Invalid timer"),
        };
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
}

impl super::TimerTrait for LockedArc<TimerGroup> {
    fn get_timer(&mut self, i: u8) -> Result<super::TimerInstance, super::TimerError> {
        let mut s = self.lock();
        let check = 1u8 << i;
        if (s.clocks_used & check) == 0 {
            s.clocks_used |= check;
            Ok(super::TimerInstance::BasicStm327f69Timer(LockedArc::new(
                Timer {
                    timer: self.clone(),
                    index: i,
                },
            )))
        } else {
            Err(super::TimerError::TimerIsAlreadyUsed)
        }
    }
}

/// An individual timer of the basic timergroup.
pub struct Timer {
    /// Reference to the hardware
    timer: LockedArc<TimerGroup>,
    /// Which timer in specific this timer is.
    index: u8,
}

impl Drop for Timer {
    fn drop(&mut self) {
        let mut t = self.timer.lock();
        t.unadjust();
        let check = 1u8 << self.index;
        t.clocks_used &= !check;
    }
}

impl Timer {
    fn delay_cycles(&self, counts_required: u64, mut t: spin::MutexGuard<'_, TimerGroup>) {
        if counts_required > 0xFFFF {
            let mut counter = 0;
            loop {
                let ccr = t.counter() as u64 + 0xFFFF;
                t.set_ccr(self.index, (ccr & 0xFFFF) as u16);
                t.start_timer();
                loop {
                    let flag = t.get_compare(self.index);
                    if flag {
                        break;
                    }
                }
                t.stop_timer();
                counter += 0xffff;
                if counter >= counts_required {
                    break;
                }
            }
        } else {
            let ccr = t.counter() as u64 + counts_required;
            t.set_ccr(self.index, (ccr & 0xFFFF) as u16);
            t.clear_compare(self.index);
            t.start_timer();
            loop {
                let flag = t.get_compare(self.index);
                if flag {
                    break;
                }
            }
            t.stop_timer();
        }
    }
}

impl super::TimerInstanceTrait for LockedArc<Timer> {
    fn delay_us(&self, us: u32) {
        let s = self.lock();
        let mut t = s.timer.lock();
        t.clock.enable_clock();

        let freq = t.clock.clock_frequency().unwrap();
        let mut prescaler = freq / 2000000;
        if prescaler > 0x10000 {
            prescaler = 0x10000;
        }
        if t.try_adjust(prescaler as u32).is_err() {
            prescaler = t.prescaler() as u64;
        }

        let counts_required3 = freq * us as u64;
        let counts_required2 = counts_required3 / 1_000_000;
        let mut counts_required = counts_required2 / prescaler;
        let count_mod = counts_required2 % prescaler;
        if count_mod != 0 {
            counts_required += 1;
        }
        s.delay_cycles(counts_required, t);
    }

    fn delay_ms(&self, ms: u32) {
        let s = self.lock();
        let mut t = s.timer.lock();
        t.clock.enable_clock();

        let freq = t.clock.clock_frequency().unwrap();
        let mut prescaler = freq / 2000;
        if prescaler > 0x10000 {
            prescaler = 0x10000;
        }
        if t.try_adjust(prescaler as u32).is_err() {
            prescaler = t.prescaler() as u64;
        }

        let counts_required3 = freq * ms as u64;
        let counts_required2 = counts_required3 / 1000;
        let mut counts_required = counts_required2 / prescaler;
        let count_mod = counts_required2 % prescaler;
        if count_mod != 0 {
            counts_required += 1;
        }
        s.delay_cycles(counts_required, t);
    }
}

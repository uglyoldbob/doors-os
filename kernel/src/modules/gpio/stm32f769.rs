//! The driver for the gpio on the the stm32f769 processor

use crate::{modules::clock::ClockProviderTrait, LockedArc};

struct GpioRegisters {
    mode: u32,
    otype: u32,
    ospeed: u32,
    pupd: u32,
    idr: u32,
    odr: u32,
    bsr: u32,
    lock: u32,
    afrl: u32,
    afrh: u32,
}

/// A gpio pin for the stm32f769 hardware
pub struct GpioPin {
    gpioref: LockedArc<Gpio>,
    i: u8,
}

impl super::GpioPinTrait for GpioPin {
    fn set_output(&mut self) {
        use super::GpioTrait;
        self.gpioref.set_output(self.i as usize);
    }

    fn write_output(&mut self, v: bool) {
        use super::GpioTrait;
        self.gpioref.write_output(self.i as usize, v);
    }
}

/// A single stm32f769 gpio module
pub struct Gpio {
    /// The hardware for enabling and disabling the gpio module clock
    cc: crate::modules::clock::ClockProvider,
    /// The index for using the rcc
    index: usize,
    /// the memory mapped registers for the hardware
    registers: &'static mut GpioRegisters,
}

impl Gpio {
    /// Construct a new gpio module with the specified address.
    pub unsafe fn new(cc: &crate::modules::clock::ClockProvider, index: usize, addr: u32) -> Self {
        Self {
            cc: cc.clone(),
            index,
            registers: &mut *(addr as *mut GpioRegisters),
        }
    }
}

impl super::GpioTrait for LockedArc<Gpio> {
    fn reset(&mut self, r: bool) {
        let s = self.lock();
        if !r {
            s.cc.enable_clock(s.index);
        } else {
            s.cc.disable_clock(s.index);
        }
    }

    fn get_pin(&self, i: usize) -> Option<super::GpioPin> {
        assert!(i < 16);
        Some(super::GpioPin::Stm32f769(GpioPin {
            gpioref: self.clone(),
            i: i as u8,
        }))
    }

    fn set_output(&mut self, i: usize) {
        assert!(i < 16);
        let mut s = self.lock();
        let mode_filter = (3u32) << (2 * i);
        let nm = unsafe { core::ptr::read_volatile(&s.registers.mode) } & !mode_filter;
        let mode = (1u32) << (2 * i);
        unsafe { core::ptr::write_volatile(&mut s.registers.mode, nm | mode) };
    }

    fn write_output(&mut self, i: usize, v: bool) {
        assert!(i < 16);
        let mut s = self.lock();
        let m = 1 << i;
        let newval = if v {
            (unsafe { core::ptr::read_volatile(&s.registers.odr) } & !m) | m
        } else {
            (unsafe { core::ptr::read_volatile(&s.registers.odr) } & !m)
        };
        unsafe {
            core::ptr::write_volatile(&mut s.registers.odr, newval);
        }
    }

    fn set_alternate(&mut self, i: usize, m: u32) {
        assert!(i < 16);
        let mut s = self.lock();
        let v = unsafe { core::ptr::read_volatile(&s.registers.mode) } & !(3 << (2 * i));
        unsafe {
            core::ptr::write_volatile(&mut s.registers.mode, v | (2 << (2 * i)));
            core::ptr::read_volatile(&s.registers.mode);
        }
        let m = m & 0xF;
        if i < 8 {
            let v = unsafe { core::ptr::read_volatile(&s.registers.afrl) } & !(0xF << (4 * i));
            unsafe {
                core::ptr::write_volatile(&mut s.registers.afrl, v | (m << (4 * i)));
            }
        } else {
            let i = i - 8;
            let v = unsafe { core::ptr::read_volatile(&s.registers.afrh) } & !(0xF << (4 * i));
            unsafe {
                core::ptr::write_volatile(&mut s.registers.afrh, v | (m << (4 * i)));
            }
        }
    }

    fn set_speed(&mut self, i: usize, speed: u32) {
        assert!(i < 16);
        let mut s = self.lock();
        let speed = speed & 3;
        let v = unsafe { core::ptr::read_volatile(&s.registers.ospeed) } & !(3 << (2 * i));
        unsafe {
            core::ptr::write_volatile(&mut s.registers.ospeed, v | (speed << (2 * i)));
        }
    }
}

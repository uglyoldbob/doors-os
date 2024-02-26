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

impl Drop for GpioPin {
    fn drop(&mut self) {
        self.gpioref.return_pin(&self);
    }
}

impl super::GpioPinTrait for GpioPin {
    fn set_output(&mut self) {
        let mut gpio = self.gpioref.lock();
        gpio.set_output(self.i as usize);
    }

    fn write_output(&mut self, v: bool) {
        let mut gpio = self.gpioref.lock();
        gpio.write_output(self.i as usize, v);
    }

    fn set_alternate(&mut self, mode: u8) {
        let mut gpio = self.gpioref.lock();
        gpio.set_alternate(self.i as usize, mode as u32);
    }

    fn set_speed(&mut self, speed: u8) {
        let mut gpio = self.gpioref.lock();
        gpio.set_speed(self.i as usize, speed as u32);
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
    /// Keeps track of which gpio are used
    used: u8,
}

impl LockedArc<Gpio> {
    /// Return a pin to the gpio provider
    fn return_pin(&self, p: &GpioPin) {
        let mut s = self.lock();
        let mask = 1 << p.i;
        if (s.used & mask) != 0 {
            s.used &= !mask;
        }
        if s.used == 0 {
            s.gpio_reset(true);
        }
    }
}

impl Gpio {
    /// Construct a new gpio module with the specified address.
    pub unsafe fn new(cc: &crate::modules::clock::ClockProvider, index: usize, addr: u32) -> Self {
        Self {
            cc: cc.clone(),
            index,
            registers: &mut *(addr as *mut GpioRegisters),
            used: 0,
        }
    }

    /// Control the reset for the gpio module
    fn gpio_reset(&self, r: bool) {
        if !r {
            self.cc.enable_clock(self.index);
        } else {
            self.cc.disable_clock(self.index);
        }
    }

    /// Set the mode of a single gpio pin to output
    fn set_output(&mut self, i: usize) {
        assert!(i < 16);
        let mode_filter = (3u32) << (2 * i);
        let nm = unsafe { core::ptr::read_volatile(&self.registers.mode) } & !mode_filter;
        let mode = (1u32) << (2 * i);
        unsafe { core::ptr::write_volatile(&mut self.registers.mode, nm | mode) };
    }

    /// The the output level of a single gpio pin
    fn write_output(&mut self, i: usize, v: bool) {
        assert!(i < 16);
        let m = 1 << i;
        let newval = if v {
            (unsafe { core::ptr::read_volatile(&self.registers.odr) } & !m) | m
        } else {
            (unsafe { core::ptr::read_volatile(&self.registers.odr) } & !m)
        };
        unsafe {
            core::ptr::write_volatile(&mut self.registers.odr, newval);
        }
    }

    /// Set the alternate mode of a single gpio pin
    fn set_alternate(&mut self, i: usize, m: u32) {
        assert!(i < 16);
        let v = unsafe { core::ptr::read_volatile(&self.registers.mode) } & !(3 << (2 * i));
        unsafe {
            core::ptr::write_volatile(&mut self.registers.mode, v | (2 << (2 * i)));
            core::ptr::read_volatile(&self.registers.mode);
        }
        let m = m & 0xF;
        if i < 8 {
            let v = unsafe { core::ptr::read_volatile(&self.registers.afrl) } & !(0xF << (4 * i));
            unsafe {
                core::ptr::write_volatile(&mut self.registers.afrl, v | (m << (4 * i)));
            }
        } else {
            let i = i - 8;
            let v = unsafe { core::ptr::read_volatile(&self.registers.afrh) } & !(0xF << (4 * i));
            unsafe {
                core::ptr::write_volatile(&mut self.registers.afrh, v | (m << (4 * i)));
            }
        }
    }

    fn set_speed(&mut self, i: usize, speed: u32) {
        assert!(i < 16);
        let speed = speed & 3;
        let v = unsafe { core::ptr::read_volatile(&self.registers.ospeed) } & !(3 << (2 * i));
        unsafe {
            core::ptr::write_volatile(&mut self.registers.ospeed, v | (speed << (2 * i)));
        }
    }
}

impl super::GpioTrait for LockedArc<Gpio> {
    fn get_pin(&self, i: usize) -> Option<super::GpioPin> {
        assert!(i < 16);
        let mut s = self.lock();
        let mask = 1 << i;
        if (s.used & mask) != 0 {
            return None;
        }
        if s.used == 0 {
            s.gpio_reset(false);
        }
        s.used |= mask;
        Some(super::GpioPin::Stm32f769(GpioPin {
            gpioref: self.clone(),
            i: i as u8,
        }))
    }
}

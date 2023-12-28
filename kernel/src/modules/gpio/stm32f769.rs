//! The driver for the gpio on the the stm32f769 processor

use alloc::sync::Arc;

use crate::{modules::clock::ClockProviderTrait, Locked};

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
    i: u8,
}

impl super::GpioPinTrait for GpioPin {
    fn set_output(&mut self) {}

    fn write_output(&mut self, v: bool) {}
}

/// A single stm32f769 gpio module
pub struct Gpio<'a> {
    /// The hardware for enabling and disabling the gpio module clock
    cc: crate::modules::clock::ClockProvider,
    /// The index for using the rcc
    index: usize,
    /// the memory mapped registers for the hardware
    registers: &'a mut GpioRegisters,
}

impl<'a> Gpio<'a> {
    /// Construct a new gpio module with the specified address.
    pub unsafe fn new(cc: &crate::modules::clock::ClockProvider, index: usize, addr: u32) -> Self {
        Self {
            cc: cc.clone(),
            index,
            registers: &mut *(addr as *mut GpioRegisters),
        }
    }
}

impl<'a> super::GpioTrait for Gpio<'a> {
    fn reset(&mut self, r: bool) {
        if !r {
            self.cc.enable_clock(self.index);
        } else {
            self.cc.disable_clock(self.index);
        }
    }

    fn get_pin(&self, i: usize) -> Option<super::GpioPin> {
        assert!(i < 16);
        None
    }

    fn set_output(&mut self, i: usize) {
        assert!(i < 16);
        let mode_filter = (3u32) << (2 * i);
        let nm = unsafe { core::ptr::read_volatile(&self.registers.mode) } & !mode_filter;
        let mode = (1u32) << (2 * i);
        unsafe { core::ptr::write_volatile(&mut self.registers.mode, nm | mode) };
    }

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
}

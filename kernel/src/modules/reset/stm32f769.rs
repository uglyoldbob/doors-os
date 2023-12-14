//! The reset controller for the stm32f769

use crate::Locked;

struct Registers {
    regs: [u32; 37],
}

/// The reset and control hardware
pub struct Module<'a> {
    registers: &'a mut Registers,
}

impl<'a> super::ResetProviderTrait for Locked<Module<'a>> {
    fn disable(&self, i: usize) {
        let mut s = self.lock();
    }

    fn enable(&self, i: usize) {
        let mut s = self.lock();
    }
}

impl<'a> crate::modules::clock::ClockProviderTrait for Locked<Module<'a>> {
    fn disable(&self, i: usize) {
        let mut s = self.lock();
        let index = i / 32;
        let i = i % 32;
        match index {
            0 => {
                let n = unsafe { core::ptr::read_volatile(&s.registers.regs[12]) } & !(1 << i);
                unsafe { core::ptr::write_volatile(&mut s.registers.regs[12], n) };
                unsafe { core::ptr::read_volatile(&s.registers.regs[12]) };
            }
            _ => {}
        }
    }

    fn enable(&self, i: usize) {
        let mut s = self.lock();
        let index = i / 32;
        let i = i % 32;
        match index {
            0 => {
                let n = unsafe { core::ptr::read_volatile(&s.registers.regs[12]) } | (1 << i);
                unsafe { core::ptr::write_volatile(&mut s.registers.regs[12], n) };
                unsafe { core::ptr::read_volatile(&s.registers.regs[12]) };
            }
            _ => {}
        }
    }
}

impl<'a> Module<'a> {
    /// Create a new peripheral with the specified address
    pub unsafe fn new(addr: u32) -> Self {
        Self {
            registers: &mut *(addr as *mut Registers),
        }
    }

    /// Disable the specified peripheral
    pub fn disable_peripheral(&mut self, i: u8) -> u32 {
        let n = unsafe { core::ptr::read_volatile(&self.registers.regs[12]) } & !(1 << i);
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[12], n) };
        unsafe { core::ptr::read_volatile(&self.registers.regs[12]) }
    }

    /// Enable the specified peripheral
    pub fn enable_peripheral(&mut self, i: u8) -> u32 {
        let n = unsafe { core::ptr::read_volatile(&self.registers.regs[12]) } | (1 << i);
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[12], n) };
        unsafe { core::ptr::read_volatile(&self.registers.regs[12]) }
    }
}

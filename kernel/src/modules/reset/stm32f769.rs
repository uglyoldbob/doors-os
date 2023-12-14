//! The reset controller for the stm32f769

use crate::Locked;

struct Registers {
    regs: [u32; 37],
}

/// The reset and control hardware
pub struct Module<'a> {
    registers: &'a mut Registers,
}

fn calc_registers(i: usize) -> (usize, u32) {
    let index = i / 32;
    let i = i % 32;
    let reg_num = match index {
        0 => 12,
        1 => 13,
        2 => 14,
        3 => 16,
        4 => 17,
        _ => panic!("Invalid group for reset enable"),
    };
    (reg_num, 1 << i)
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
        let (reg_num, i) = calc_registers(i);

        let n = unsafe { core::ptr::read_volatile(&s.registers.regs[reg_num]) } & !i;
        unsafe { core::ptr::write_volatile(&mut s.registers.regs[reg_num], n) };
        unsafe { core::ptr::read_volatile(&s.registers.regs[reg_num]) };
    }

    fn enable(&self, i: usize) {
        let mut s = self.lock();
        let (reg_num, i) = calc_registers(i);
        let n = unsafe { core::ptr::read_volatile(&s.registers.regs[reg_num]) } | i;
        unsafe { core::ptr::write_volatile(&mut s.registers.regs[reg_num], n) };
        unsafe { core::ptr::read_volatile(&s.registers.regs[reg_num]) };
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

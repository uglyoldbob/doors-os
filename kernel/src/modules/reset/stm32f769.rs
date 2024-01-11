//! The reset controller for the stm32f769

use crate::LockedArc;

struct Registers {
    regs: [u32; 37],
}

/// The reset and control hardware
pub struct Module<'a> {
    registers: &'a mut Registers,
}

fn calc_clock_register(i: usize) -> (usize, u32) {
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

impl<'a> super::ResetProviderTrait for LockedArc<Module<'a>> {
    fn disable(&self, _i: usize) {}

    fn enable(&self, _i: usize) {}
}

impl<'a> Module<'a> {
    /// Create a new peripheral with the specified address
    pub unsafe fn new(addr: u32) -> Self {
        Self {
            registers: &mut *(addr as *mut Registers),
        }
    }

    /// Set the dividers for the apb clocks
    pub fn apb_dividers(&mut self, d1: u32, d2: u32) {
        let d1 = match d1 {
            0 => 1,
            2 => 4,
            4 => 5,
            8 => 6,
            16 => 7,
            _ => panic!("Invalid divider"),
        };
        let d2 = match d2 {
            0 => 1,
            2 => 4,
            4 => 5,
            8 => 6,
            16 => 7,
            _ => panic!("Invalid divider"),
        };
        let v = (d2 & 7) << 13 | (d1 & 7) << 10;
        let n = unsafe { core::ptr::read_volatile(&self.registers.regs[2]) } & !(0x3f << 10);
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[2], n | v) };
        unsafe { core::ptr::read_volatile(&self.registers.regs[2]) };
    }

    /// Set the mco1 divider to sysclk / 5.
    pub fn set_mco1_pll(&mut self) {
        let n = unsafe { core::ptr::read_volatile(&self.registers.regs[2]) } & !(0x07600000);
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[2], n | 0x07600000) };
        unsafe { core::ptr::read_volatile(&self.registers.regs[2]) };
    }

    /// Disable the specified peripheral
    pub fn disable_peripheral(&mut self, i: u8) -> u32 {
        let (reg, i) = calc_clock_register(i as usize);
        let n = unsafe { core::ptr::read_volatile(&self.registers.regs[reg]) } & !i;
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[reg], n) };
        unsafe { core::ptr::read_volatile(&self.registers.regs[reg]) }
    }

    /// Enable the specified peripheral
    pub fn enable_peripheral(&mut self, i: u8) -> u32 {
        let (reg, i) = calc_clock_register(i as usize);
        let n = unsafe { core::ptr::read_volatile(&self.registers.regs[reg]) } | i;
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[reg], n) };
        unsafe { core::ptr::read_volatile(&self.registers.regs[reg]) }
    }

    /// Enable the hse bypass to allow for a direct clock input on the hse
    pub fn set_hse_bypass(&mut self, s: bool) {
        let mut newval = unsafe { core::ptr::read_volatile(&self.registers.regs[0]) } & !(1 << 18);
        if s {
            newval |= 1 << 18;
        }
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[0], newval) };
    }

    /// Set the HSE clock
    pub fn set_hse(&mut self, s: bool) {
        let mut newval = unsafe { core::ptr::read_volatile(&self.registers.regs[0]) } & !(1 << 16);
        if s {
            newval |= 1 << 16;
        }
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[0], newval) };
    }

    /// Is the hse ready?
    pub fn hse_ready(&self) -> bool {
        let val = unsafe { core::ptr::read_volatile(&self.registers.regs[0]) };
        (val & (1 << 17)) != 0
    }

    /// Set the HSI clock
    pub fn set_hsi(&mut self, s: bool) {
        let mut newval = unsafe { core::ptr::read_volatile(&self.registers.regs[0]) } & !(1 << 0);
        if s {
            newval |= 1 << 0;
        }
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[0], newval) };
    }

    /// Is the hsi ready?
    pub fn hsi_ready(&self) -> bool {
        let val = unsafe { core::ptr::read_volatile(&self.registers.regs[0]) };
        (val & (1 << 1)) != 0
    }

    /// Set the status of mux1, which sets the input clock for the main pll and the i2s pll.
    /// True means select the HSE oscillator, false means select the HSI oscillator
    pub fn set_mux1(&mut self, v: bool) {
        let mut newval = unsafe { core::ptr::read_volatile(&self.registers.regs[1]) } & !(1 << 22);
        if v {
            newval |= 1 << 22;
        }
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[1], newval) };
    }

    /// Get the status of the mux1 switch
    pub fn get_mux1(&self) -> bool {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[1]) } & (1 << 22);
        v != 0
    }

    /// Get the divisor for the main divider
    pub fn get_divider1(&self) -> u32 {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[1]) } & 0x3F;
        v
    }

    /// Set the divisor for the main divider
    pub fn set_divider1(&mut self, d: u32) {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[1]) } & !0x3F;
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[1], v | (d & 0x3F)) };
    }

    /// Set the multiplier for the main pll
    pub fn set_multiplier1(&mut self, d: u32) {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[1]) } & !0x7FC0;
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[1], v | ((d << 6) & 0x7FC0)) };
    }

    /// Get the multiplier for the main pll
    pub fn get_multiplier1(&self) -> u32 {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[1]) };
        (v >> 6) & 0x1FF
    }

    /// Set the multiplier for the second pll
    pub fn set_multiplier2(&mut self, d: u32) {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[33]) } & !0x7FC0;
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[33], v | ((d << 6) & 0x7FC0)) };
    }

    /// Get the multiplier for the second pll
    pub fn get_multiplier2(&self) -> u32 {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[33]) } & 0x7FC0;
        (v >> 6) & 0x1FF
    }

    /// Set the multiplier for the third pll
    pub fn set_multiplier3(&mut self, d: u32) {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[34]) } & !0x7FC0;
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[34], v | ((d << 6) & 0x7FC0)) };
    }

    /// Get the multiplier for the third pll
    pub fn get_multiplier3(&self) -> u32 {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[34]) } & 0x7FC0;
        (v >> 6) & 0x1FF
    }

    /// Is the main pll ready and locked?
    pub fn main_pll_locked(&self) -> bool {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[0]) } & (1 << 25);
        v != 0
    }

    /// Is the second pll ready and locked?
    pub fn second_pll_locked(&self) -> bool {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[0]) } & (1 << 27);
        v != 0
    }

    /// Is the third pll ready and locked?
    pub fn third_pll_locked(&self) -> bool {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[0]) } & (1 << 29);
        v != 0
    }

    /// Set the main pll enable bit
    pub fn set_main_pll(&mut self, v: bool) {
        let mut newval = unsafe { core::ptr::read_volatile(&self.registers.regs[0]) } & !(1 << 24);
        if v {
            newval |= 1 << 24;
        }
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[0], newval) };
    }

    /// Set the second pll enable bit
    pub fn set_second_pll(&mut self, v: bool) {
        let mut newval = unsafe { core::ptr::read_volatile(&self.registers.regs[0]) } & !(1 << 26);
        if v {
            newval |= 1 << 26;
        }
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[0], newval) };
    }

    /// Set the third pll enable bit
    pub fn set_third_pll(&mut self, v: bool) {
        let mut newval = unsafe { core::ptr::read_volatile(&self.registers.regs[0]) } & !(1 << 28);
        if v {
            newval |= 1 << 28;
        }
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[0], newval) };
    }

    /// The the mux for the sysclk
    pub fn get_mux_sysclk(&self) -> u8 {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[2]) } & 3;
        v as u8
    }

    /// Set the mux for the sysclk generation
    pub fn set_mux_sysclk(&mut self, v: u8) {
        let mut newval = unsafe { core::ptr::read_volatile(&self.registers.regs[2]) } & !3;
        newval |= v as u32 & 3;
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[2], newval) };
    }

    /// Set the specified main pll divisor
    pub fn set_main_pll_divisor(&mut self, i: usize, d: u8) {
        let (val, mask, shift) = match i {
            0 => {
                let divisor = match d {
                    2 => 0,
                    4 => 1,
                    6 => 2,
                    8 => 3,
                    _ => panic!("Cannot set main pll divisor"),
                };
                (divisor, 3, 16)
            }
            1 => (d, 0xF, 24),
            2 => (d, 7, 28),
            _ => {
                panic!("Invalid pll output specified");
            }
        };
        let mut newval =
            unsafe { core::ptr::read_volatile(&self.registers.regs[1]) } & !(mask << shift);
        newval |= (mask & val as u32) << shift;
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[1], newval) };
    }

    /// Get the specified main pll dividor
    pub fn get_main_pll_divisor(&self, i: usize) -> u8 {
        let (mask, shift, shift2) = match i {
            0 => (3, 16, 15),
            1 => (0xF, 24, 24),
            2 => (7, 28, 28),
            _ => {
                panic!("Invalid pll output specified");
            }
        };
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[1]) } & (mask << shift);
        let mut div = (v >> shift2) as u8;
        if i == 0 {
            div = 2 * (div + 1);
        }
        div
    }

    /// Set the specified second pll divisor
    pub fn set_second_pll_divisor(&mut self, i: usize, d: u8) {
        let (val, mask, shift) = match i {
            0 => {
                let divisor = match d {
                    2 => 0,
                    4 => 1,
                    6 => 2,
                    8 => 3,
                    _ => panic!("Cannot set main pll divisor"),
                };
                (divisor, 3, 16)
            }
            1 => (d, 0xF, 24),
            2 => (d, 7, 28),
            _ => {
                panic!("Invalid pll output specified");
            }
        };
        let mut newval =
            unsafe { core::ptr::read_volatile(&self.registers.regs[33]) } & !(mask << shift);
        newval |= (mask & val as u32) << shift;
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[33], newval) };
    }

    /// Get the specified second pll dividor
    pub fn get_second_pll_divisor(&self, i: usize) -> u8 {
        let (mask, shift, shift2) = match i {
            0 => (3, 16, 15),
            1 => (0xF, 24, 24),
            2 => (7, 28, 28),
            _ => {
                panic!("Invalid pll output specified");
            }
        };
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[33]) } & (mask << shift);
        (v >> shift2) as u8
    }

    /// Set the specified second pll divisor
    pub fn set_third_pll_divisor(&mut self, i: usize, d: u8) {
        let (val, mask, shift) = match i {
            0 => {
                let divisor = match d {
                    2 => 0,
                    4 => 1,
                    6 => 2,
                    8 => 3,
                    _ => panic!("Cannot set main pll divisor"),
                };
                (divisor, 3, 16)
            }
            1 => (d, 0xF, 24),
            2 => {
                assert!(d > 1);
                (d, 7, 28)
            }
            _ => {
                panic!("Invalid pll output specified");
            }
        };
        let mut newval =
            unsafe { core::ptr::read_volatile(&self.registers.regs[34]) } & !(mask << shift);
        newval |= (mask & val as u32) << shift;
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[34], newval) };
    }

    /// Get the specified second pll dividor
    pub fn get_third_pll_divisor(&self, i: usize) -> u8 {
        let (mask, shift, shift2) = match i {
            0 => (3, 16, 15),
            1 => (0xF, 24, 24),
            2 => (7, 28, 28),
            _ => {
                panic!("Invalid pll output specified");
            }
        };
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[34]) } & (mask << shift);
        (v >> shift2) as u8
    }

    /// Set the usart mux data for the specified usart module. See get_usart_mux
    pub fn set_usart_mux(&mut self, i: u8, v: u8) {
        let mask = 3u32 << (2 * i);
        let ov = unsafe { core::ptr::read_volatile(&self.registers.regs[36]) } & !mask;
        let nm = ((v & 3) as u32) << (2 * i);
        unsafe { core::ptr::write_volatile(&mut self.registers.regs[36], ov | nm) };
    }

    /// Get the usart mux data for the specified usart module.
    /// 0 = APB clock.
    /// 1 = sysclock.
    /// 2 = hsi clock.
    /// 3 = lse clock.
    pub fn get_usart_mux(&self, i: u8) -> u8 {
        let v = unsafe { core::ptr::read_volatile(&self.registers.regs[36]) };
        ((v >> (2 * i)) & 3) as u8
    }
}

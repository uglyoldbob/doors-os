//! Dsi related code for the stm32f769

use crate::modules::clock::{ClockProviderTrait, ClockRefTrait};
use crate::modules::clock::{PllDividerErr, PllVcoSetError};
use crate::LockedArc;

/// The memory mapped registers of the ltdc hardware
struct LtdcRegisters {
    /// The registers
    regs: [u8; 5],
}

/// The ltdc module of the stm32f769 processor
struct Ltdc {
    /// The clock provider
    cc: crate::modules::clock::ClockProvider,
    /// The memory mapped registers
    regs: &'static mut LtdcRegisters,
}

impl Ltdc {
    /// Build a new object
    pub unsafe fn new(cc: &crate::modules::clock::ClockProvider, addr: usize) -> Self {
        Self {
            cc: cc.clone(),
            regs: &mut *(addr as *mut LtdcRegisters),
        }
    }

    /// Enable the clock input for the hardware
    pub fn enable(&self) {
        self.cc.enable(4 * 32 + 26);
    }

    /// Disable the clock input for the hardware
    pub fn disable(&self) {
        self.cc.disable(4 * 32 + 26);
    }
}

/// The memory mapped registers for the dsi hardware
struct DsiRegisters {
    /// The registers
    regs: [u32; 269],
}

struct ModuleInternals {
    /// The registers for the hardware
    regs: &'static mut DsiRegisters,
}

/// The dsi hardware implementation
#[derive(Clone)]
pub struct Module {
    /// The hardware for enabling and disabling the clock
    cc: crate::modules::clock::ClockProvider,
    /// The input clocks. 0 is the optional clock for the byte clock, 1 is the input to the pll
    iclk: [crate::modules::clock::ClockRef; 2],
    // The internals for the hardware
    internals: LockedArc<ModuleInternals>,
    /// The related ltdc hardware
    ltdc: LockedArc<Ltdc>,
}

impl super::MipiDsiTrait for Module {
    fn enable(&self) {
        self.cc.enable(4 * 32 + 27);
        let ltdc = self.ltdc.lock();
        ltdc.enable();
    }

    fn disable(&self) {
        let ltdc = self.ltdc.lock();
        ltdc.disable();
        drop(ltdc);
        self.cc.disable(4 * 32 + 27);
    }
}

impl Module {
    /// Create a new hardware instance.
    /// iclk is a slice of the two clocks for the dsi. Index 0 is for the clock that leads to the dsi byte clock, index 1 is for the pll input.
    pub unsafe fn new(
        cc: &crate::modules::clock::ClockProvider,
        iclk: [&crate::modules::clock::ClockRef; 2],
        addr: usize,
    ) -> Self {
        let nclk: [crate::modules::clock::ClockRef; 2] = [iclk[0].clone(), iclk[1].clone()];
        Self {
            cc: cc.clone(),
            internals: LockedArc::new(ModuleInternals {
                regs: &mut *(addr as *mut DsiRegisters),
            }),
            ltdc: LockedArc::new(Ltdc::new(cc, 0x4001_6800)),
            iclk: nclk,
        }
    }

    fn get_input_divider(&self) -> u32 {
        let internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) };
        let val = (v >> 11) & 0xF;
        if val == 0 {
            1
        } else {
            val
        }
    }

    /// Set the vco multiplier of the pll
    fn set_multiplier(&self, d: u32) {
        let mut internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) };
        let newval = (v & !0x1FC) | ((d as u32 & 0x7F) << 2);
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[268], newval) };
    }

    /// Get the vco multiplier of the pll
    fn get_multiplier(&self) -> u32 {
        let internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) };
        (v >> 2) & 0x7F
    }
}

impl crate::modules::clock::ClockProviderTrait for Module {
    /// Enable the pll
    fn enable(&self, _i: usize) {
        let mut internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) };
        let newval = v | 1;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[268], newval) };
    }

    /// Disable the pll
    fn disable(&self, _i: usize) {
        let mut internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) };
        let newval = v & !1;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[268], newval) };
    }

    fn is_ready(&self, _i: usize) -> bool {
        let internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[259]) };
        (v & 1 << 8) != 0
    }

    fn frequency(&self, i: usize) -> Option<u64> {
        if let Some(fin) = self.iclk[1].frequency() {
            let id = self.get_input_divider();
            let vco_mul = self.get_multiplier();
            let divider = crate::modules::clock::PllProviderTrait::get_post_divider(self, i) as u64;
            let fout = (fin * vco_mul as u64) / (id as u64 * divider);
            return Some(fout);
        } else {
            return None;
        }
    }
}

impl crate::modules::clock::PllProviderTrait for Module {
    fn get_input_frequency(&self) -> Option<u64> {
        self.iclk[1].frequency()
    }

    fn set_input_divider(&self, d: u32) -> Result<(), crate::modules::clock::PllDividerErr> {
        if (d & !7) != 0 {
            return Err(PllDividerErr::ImpossibleDivisor);
        }
        if let Some(fin) = self.iclk[1].frequency() {
            if !(4_000_000..=100_000_000).contains(&fin) {
                return Err(PllDividerErr::InputFrequencyOutOfRange);
            }
            let internal_freq = fin / d as u64;
            if !(4_000_000..=25_000_000).contains(&internal_freq) {
                return Err(PllDividerErr::InputFrequencyOutOfRange);
            }
        } else {
            return Err(PllDividerErr::UnknownInputFrequency);
        }
        let mut internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) };
        let newval = (v & !0x7800) | (d & 0xF) << 11;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[268], newval) };
        Ok(())
    }

    /// This divider accounts for the divide by 2 factor already present in the dsi pll.
    fn set_post_divider(&self, i: usize, d: u32) -> Result<u32, PllDividerErr> {
        let divider = match d {
            2 => 0,
            4 => 1,
            8 => 2,
            16 => 3,
            _ => return Err(PllDividerErr::ImpossibleDivisor),
        };

        let id = self.get_input_divider();
        let vco_mul = self.get_multiplier();
        if let Some(fin) = self.iclk[1].frequency() {
            let vco_freq = fin as u32 * vco_mul as u32;
            let fout = vco_freq / (id as u32 * d as u32);
            if !(31_250_000..=82_500_000).contains(&fout) {
                return Err(PllDividerErr::InputFrequencyOutOfRange);
            }
        } else {
            return Err(PllDividerErr::UnknownInputFrequency);
        }

        let mut internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) };
        let newval = (v & !0x30000) | (divider as u32) << 2;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[268], newval) };
        Ok((unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) } >> 16) & 0x3)
    }

    fn get_post_divider(&self, _i: usize) -> u32 {
        let internals = self.internals.lock();
        let d = (unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) } >> 16) & 3;
        match d {
            0 => 2,
            1 => 4,
            2 => 8,
            3 => 16,
            _ => unreachable!(),
        }
    }

    fn set_vco_frequency(&self, f: u64) -> Result<(), PllVcoSetError> {
        if !(500_000_000..=1_000_000_000).contains(&f) {
            return Err(PllVcoSetError::FrequencyOutOfRange);
        }

        if let Some(fin) = self.iclk[1].frequency() {
            let fin = fin / self.get_input_divider() as u64;
            let multiplier = f / fin;
            self.set_multiplier(multiplier as u32);
            Ok(())
        } else {
            Err(PllVcoSetError::UnknownInputFrequency)
        }
    }
}

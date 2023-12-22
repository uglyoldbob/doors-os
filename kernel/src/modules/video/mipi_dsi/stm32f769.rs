//! Dsi related code for the stm32f769

use crate::modules::clock::{ClockProviderTrait, ClockRefTrait};
use crate::modules::clock::{PllDividerErr, PllVcoSetError};
use crate::LockedArc;

/// The memory mapped registers of the ltdc hardware
struct LtdcRegisters {
    /// The registers
    regs: [u32; 82],
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

    /// Enable the ltdc hardware
    pub fn enable(&mut self) {
        let v = unsafe { core::ptr::read_volatile(&self.regs.regs[6]) };
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[6], v | 1) };
    }

    /// Enable the clock input for the hardware
    pub fn enable_clock(&self) {
        self.cc.enable(4 * 32 + 26);
    }

    /// Disable the clock input for the hardware
    pub fn disable_clock(&self) {
        self.cc.disable(4 * 32 + 26);
    }

    pub fn configure(&mut self, resolution: &super::super::ScreenResolution) {
        let v = (resolution.hsync as u32 - 1) << 16 | (resolution.vsync as u32 - 1);
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[2], v) };

        let v = (resolution.h_b_porch as u32 + resolution.hsync as u32 - 1) << 16
            | (resolution.v_b_porch as u32 + resolution.vsync as u32 - 1);
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[3], v) };

        let v = (resolution.width as u32 + resolution.h_b_porch as u32 + resolution.hsync as u32
            - 1)
            << 16
            | (resolution.height as u32 + resolution.v_b_porch as u32 + resolution.vsync as u32
                - 1);
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[4], v) };

        let v = (resolution.h_f_porch as u32
            + resolution.width as u32
            + resolution.h_b_porch as u32
            + resolution.hsync as u32
            - 1)
            << 16
            | (resolution.v_f_porch as u32
                + resolution.height as u32
                + resolution.v_b_porch as u32
                + resolution.vsync as u32
                - 1);
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[5], v) };

        //trigger immediate load
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[9], 1) };
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

/// The dsi hardware implementation. The pll of the stm32f769 is integrated into this struct functionality.
#[derive(Clone)]
pub struct Module {
    /// The hardware for enabling and disabling the clock
    cc: alloc::boxed::Box<crate::modules::clock::ClockProvider>,
    /// The input clocks. 0 is the optional clock for the byte clock, 1 is the input to the pll
    iclk: [alloc::boxed::Box<crate::modules::clock::ClockRef>; 2],
    // The internals for the hardware
    internals: LockedArc<ModuleInternals>,
    /// The related ltdc hardware
    ltdc: LockedArc<Ltdc>,
}

impl super::MipiDsiTrait for Module {
    fn enable(&self, config: super::MipiDsiConfig, resolution: super::super::ScreenResolution) {
        self.cc.enable(4 * 32 + 27);
        let mut ltdc = self.ltdc.lock();
        ltdc.enable_clock();

        ltdc.configure(&resolution);

        self.enable_regulator();

        //configure the pll
        let dsi_pll = crate::modules::clock::PllProvider::Stm32f769DsiPll(self.clone());
        loop {
            if crate::modules::clock::PllProviderTrait::set_input_divider(&dsi_pll, 1).is_ok() {
                break;
            }
        }
        loop {
            if crate::modules::clock::PllProviderTrait::set_vco_frequency(&dsi_pll, 750_000_000)
                .is_ok()
            {
                break;
            }
        }
        loop {
            if crate::modules::clock::PllProviderTrait::set_post_divider(&dsi_pll, 0, 16).is_ok() {
                break;
            }
        }

        //enable and wait for the pll
        let pll_provider = crate::modules::clock::ClockProvider::Stm32f769DsiPll(self.clone());
        crate::modules::clock::ClockProvider::enable(&pll_provider, 0);
        while !crate::modules::clock::ClockProvider::is_ready(&pll_provider, 0) {}

        let val = 4_000_000_000 / config.link_speed;
        self.set_dphy_link(val);

        // set the number of lanes (only 1 or 2 lanes supported here)
        let mut internals = self.internals.lock();

        // set the stop wait time for stopping high speed transmissions on dsi? (bits 16-23)
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[41]) } & 0xFF00;
        let nlanes = ((config.num_lanes - 1) & 1) as u32;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[41], v | nlanes) };

        //set automatic clock lane control and clock control
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[37]) } & 0xFF00;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[37], v | 3) };

        //set transition time for dsi clock signal?
        //set transition time for dsi data signals?
        //set read time for dsi data signals?

        //TODO put in actual values here
        let ockdiv = 255;
        let eckdiv = 30;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[2], (ockdiv << 8) | eckdiv) };

        let pcrval = 0x1f;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[11], pcrval) };

        //set vcid for the display
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[3], config.vcid as u32 & 3) };

        // setup WCFGR with DSIM, COLMUX, TESRC, TEPOL, AR, and VSPOL?

        //setup VMCR, VPCR, VCCR, VNPCR, VLCR, VHSACR, VHBPCR, VVSACR, VVBPCR, VVFPCR, VVACR registers

        // pixels per packet (VPCR)
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[15], 200) };
        //chunks per line
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[16], 4) };
        // size of null packet
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[17], 1) };
        // horizontal sync active length
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[18], resolution.width as u32) };
        //horizontal back porch length
        unsafe {
            core::ptr::write_volatile(&mut internals.regs.regs[19], resolution.h_b_porch as u32)
        };
        //TODO calculate the number here
        let v = (resolution.h_b_porch + resolution.h_f_porch + resolution.width + resolution.hsync)
            as u32;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[20], v) };
        //vsync length
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[21], resolution.vsync as u32) };
        //vertical back porch length
        unsafe {
            core::ptr::write_volatile(&mut internals.regs.regs[22], resolution.v_b_porch as u32)
        };
        //vertical front porch duration
        unsafe {
            core::ptr::write_volatile(&mut internals.regs.regs[23], resolution.v_f_porch as u32)
        };
        //number of vertical lines
        unsafe {
            core::ptr::write_volatile(&mut internals.regs.regs[24], resolution.height as u32)
        };

        //enable data and clock
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[40]) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[40], v | (3 << 1)) };

        //enable dsi host
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[1], 1) };

        //enable dsi wrapper
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[257]) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[257], v | (1 << 3)) };

        ltdc.enable();

        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[257]) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[257], v | (1 << 2)) };
        drop(internals);
    }

    fn disable(&self) {
        let ltdc = self.ltdc.lock();
        ltdc.disable_clock();
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
        let nclk: [alloc::boxed::Box<crate::modules::clock::ClockRef>; 2] = [
            alloc::boxed::Box::new(iclk[0].clone()),
            alloc::boxed::Box::new(iclk[1].clone()),
        ];
        Self {
            cc: alloc::boxed::Box::new(cc.clone()),
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

    /// Enable the voltage regulator
    fn enable_regulator(&self) {
        let mut internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) };
        let newval = v | (1 << 24);
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[268], newval) };
    }

    /// Set the dphy link speed
    fn set_dphy_link(&self, v: u64) {
        let mut internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[262]) };
        let newval = (v & !0x3F) | (v & 0x3F);
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[262], newval) };
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

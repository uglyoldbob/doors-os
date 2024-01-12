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

    /// do some debugging by dumping all registers
    pub fn debug(&self) {
        let mut d = crate::DEBUG_STUFF.lock();
        for (i, d) in d.iter_mut().enumerate() {
            *d = unsafe { core::ptr::read_volatile(&self.regs.regs[i]) }
        }
        drop(d);
    }

    /// Enable the ltdc hardware
    pub fn enable(&mut self) {
        let v = unsafe { core::ptr::read_volatile(&self.regs.regs[6]) };
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[6], v | 1) };
        unsafe { core::ptr::read_volatile(&self.regs.regs[6]) };
    }

    /// Enable the clock input for the hardware
    pub fn enable_clock(&self) {
        self.cc.enable_clock(4 * 32 + 26);
    }

    /// Disable the clock input for the hardware
    pub fn disable_clock(&self) {
        self.cc.disable_clock(4 * 32 + 26);
    }

    pub fn configure(&mut self, resolution: &super::super::ScreenResolution) {
        self.enable();

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

        unsafe { core::ptr::write_volatile(&mut self.regs.regs[5], v) };

        //layer 1 stuff
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[33], 1) };
        unsafe {
            core::ptr::write_volatile(&mut self.regs.regs[34], (resolution.width as u32) << 16)
        };
        unsafe {
            core::ptr::write_volatile(&mut self.regs.regs[35], (resolution.height as u32) << 16)
        };
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[37], 1) };
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[39], 0xFF424242) };
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[43], 0x2002_0000) };
        unsafe {
            core::ptr::write_volatile(
                &mut self.regs.regs[44],
                (resolution.width as u32 * 3) << 16 | (3 + resolution.width as u32 * 3),
            )
        };
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[45], 480) };

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

impl ModuleInternals {
    fn command_fifo_empty(&self) -> bool {
        let v = unsafe { core::ptr::read_volatile(&self.regs.regs[29]) };
        (v & (1 << 0)) != 0
    }

    fn wait_command_fifo_empty(&self) {
        loop {
            let v = unsafe { core::ptr::read_volatile(&self.regs.regs[29]) };
            if (v & 1) != 0 {
                break;
            }
        }
    }

    fn simple_command_write(&mut self, channel: u8, cmd: u16, data: &[u8]) {
        self.wait_command_fifo_empty();
        let v: u32 = 0x15 | (channel as u32 & 3) << 6 | ((cmd & 0xFF) as u32) << 16;
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[27], v) };

        self.wait_command_fifo_empty();
        let ta = [(cmd >> 8) as u8];
        let v = ta.iter();
        let v2 = data.iter();
        let v3 = v.chain(v2);

        let mut index = 0;
        let mut val: u32 = 0;
        for (i, d) in v3.enumerate() {
            val |= (*d as u32) << (8 * index);
            if index == 3 {
                unsafe { core::ptr::write_volatile(&mut self.regs.regs[28], val) };
                val = 0;
                index = 0;
            } else {
                index += 1;
            }
        }
        if index != 0 {
            unsafe { core::ptr::write_volatile(&mut self.regs.regs[28], val) };
            val = 0;
            index = 0;
        }
        let len: u32 = data.len() as u32 + 1;
        let v: u32 = 0x39 | (channel as u32 & 3) << 6 | (len & 0xFFFF) << 8;
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[27], v) };
    }
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
    fn enable(&self, config: &super::MipiDsiConfig, resolution: &super::super::ScreenResolution) {
        self.cc.enable_clock(4 * 32 + 27);
        let mut ltdc = self.ltdc.lock();
        ltdc.enable_clock();

        ltdc.configure(&resolution);

        self.enable_regulator();

        loop {
            if self.regulator_ready() {
                break;
            }
        }

        //configure the pll
        let dsi_pll = crate::modules::clock::Pll::Stm32f769DsiPll(self.clone());
        loop {
            if crate::modules::clock::PllTrait::set_input_divider(&dsi_pll, 1).is_ok() {
                break;
            }
        }
        loop {
            if crate::modules::clock::PllTrait::set_vco_frequency(&dsi_pll, 500_000_000).is_ok() {
                break;
            }
        }
        loop {
            if crate::modules::clock::PllTrait::set_post_divider(&dsi_pll, 0, 2).is_ok() {
                break;
            }
        }

        //enable and wait for the pll
        let pll_provider = crate::modules::clock::ClockProvider::Stm32f769DsiPll(self.clone());
        crate::modules::clock::ClockProvider::enable_clock(&pll_provider, 0);
        while !crate::modules::clock::ClockProvider::clock_is_ready(&pll_provider, 0) {}

        let val = 4_000_000_000 / config.link_speed;
        self.set_dphy_link(val);

        let mut internals = self.internals.lock();

        // set the stop wait time for stopping high speed transmissions on dsi? (bits 16-23)
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[41]) } & 0xFF00;
        let nlanes = ((config.num_lanes - 1) & 1) as u32;
        // set the number of lanes (only 1 or 2 lanes supported here)
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[41], 0xa00 | nlanes) };

        //set automatic clock lane control and clock control
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[37], 1) };

        // set max timeouts
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[30], 0xffffffff) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[31], 0xffff) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[32], 0xffff) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[33], 0x100ffff) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[34], 0xffff) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[35], 0xffff) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[38], 0x230023) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[39], 0x23230000) };

        //set transition time for dsi clock signal?
        //set transition time for dsi data signals?
        //set read time for dsi data signals?

        //TODO put in actual values here
        let ockdiv = 0;
        let eckdiv = 4;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[2], (ockdiv << 8) | eckdiv) };

        let pcrval = 0x4;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[11], pcrval) };

        //set vcid for the display
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[3], config.vcid as u32 & 3) };

        //video mode
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[13], 0) };
        //test pattern generator
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[14], 0x1010001) };

        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[25], 200) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[26], 0) };

        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[256], 1 << 6) };

        // setup WCFGR with DSIM, COLMUX, TESRC, TEPOL, AR, and VSPOL?

        //setup VMCR, VPCR, VCCR, VNPCR, VLCR, VHSACR, VHBPCR, VVSACR, VVBPCR, VVFPCR, VVACR registers

        // pixels per packet (VPCR)
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[15], resolution.width as u32) };
        //chunks per line
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[16], 2) };
        // size of null packet
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[17], 1) };
        // horizontal sync active length
        unsafe {
            core::ptr::write_volatile(&mut internals.regs.regs[18], 16 * resolution.width as u32)
        };
        //horizontal back porch length
        unsafe {
            core::ptr::write_volatile(
                &mut internals.regs.regs[19],
                16 * resolution.h_b_porch as u32,
            )
        };
        //TODO calculate the number here
        let v = (resolution.h_b_porch + resolution.h_f_porch + resolution.width + resolution.hsync)
            as u32;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[20], v * 16) };
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

        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[25], resolution.width as u32) };

        //enable data and clock
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[40]) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[40], v | (3 << 1)) };

        //enable dsi host
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[1], 1) };

        //enable dsi wrapper
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[257]) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[257], v | (1 << 3)) };

        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[64], 0x101) };

        //TODO: move these function calls to another layer specific to the display
        internals.simple_command_write(0, 0xff00, &[0x80, 9, 1]);
        internals.simple_command_write(0, 0xff80, &[0x80, 9]);
        internals.simple_command_write(0, 0xc480, &[0x30]);
        todo!("Delay 10 milliseconds");
        internals.simple_command_write(0, 0xc48a, &[0x40]);
        todo!("Delay 10 milliseconds");
        internals.simple_command_write(0, 0xc5b1, &[0xa9]);
        internals.simple_command_write(0, 0xc591, &[0x34]);
        internals.simple_command_write(0, 0xc0b4, &[0x50]);
        internals.simple_command_write(0, 0xd900, &[0x4e]);
        internals.simple_command_write(0, 0xc181, &[0x66]); //65 hz display frequency
        internals.simple_command_write(0, 0xc592, &[1]);
        internals.simple_command_write(0, 0xc595, &[0x34]);
        internals.simple_command_write(0, 0xc594, &[0x33]);
        internals.simple_command_write(0, 0xd800, &[0x79, 0x79]);
        internals.simple_command_write(0, 0xc0a3, &[0x1b]);
        internals.simple_command_write(0, 0xc582, &[0x83]);
        internals.simple_command_write(0, 0xc480, &[0x83]);
        internals.simple_command_write(0, 0xc1a1, &[0x0e]);
        internals.simple_command_write(0, 0xb3a6, &[0, 1]);
        internals.simple_command_write(0, 0xce80, &[0x85, 1, 0, 0x84, 1, 0]);
        internals.simple_command_write(0, 0xcea0, &[0x18, 4, 3, 0x39, 0, 0, 0, 0x18, 3, 3, 0x3a, 0, 0, 0]);
        internals.simple_command_write(0, 0xceb0, &[0x18, 2, 3, 0x3b, 0, 0, 0, 0x18, 1, 3, 0x3c, 0, 0, 0]);
        internals.simple_command_write(0, 0xcfc0, &[0x1, 1, 0x20, 0x20, 0, 0, 1, 2, 0, 0]);
        internals.simple_command_write(0, 0xcfd0, &[0]);
        internals.simple_command_write(0, 0xcb80, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        internals.simple_command_write(0, 0xcb90, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        internals.simple_command_write(0, 0xcba0, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        internals.simple_command_write(0, 0xcbb0, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        internals.simple_command_write(0, 0xcbc0, &[0, 4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        internals.simple_command_write(0, 0xcbe0, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        internals.simple_command_write(0, 0xcbf0, &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        internals.simple_command_write(0, 0xcc80, &[0, 0x26, 9, 0xb, 1, 0x25, 0, 0, 0, 0]);
        internals.simple_command_write(0, 0xcc90, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x26, 0xa, 0xc, 2]);
        internals.simple_command_write(0, 0xcca0, &[0x25, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        internals.simple_command_write(0, 0xccb0, &[0, 0x25, 0xc, 0xa, 2, 0x26, 0, 0, 0, 0]);
        internals.simple_command_write(0, 0xccc0, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x25, 0xb, 9, 1]);
        internals.simple_command_write(0, 0xccd0, &[0x26, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        internals.simple_command_write(0, 0xc581, &[0x66]);
        internals.simple_command_write(0, 0xf5b6, &[6]);
        internals.simple_command_write(0, 0xe100, &[0, 9, 0xf, 0xe, 7, 0x10, 0xb, 0xa, 4, 7, 0xb, 8, 0xf, 0x10, 0xa, 1]);
        internals.simple_command_write(0, 0xe200, &[0, 9, 0xf, 0xe, 7, 0x10, 0xb, 0xa, 4, 7, 0xb, 8, 0xf, 0x10, 0xa, 1]);
        internals.simple_command_write(0, 0xff00, &[0xff, 0xff, 0xff]);

        todo!("Finish display initialization commands");

        ltdc.debug();
        ltdc.enable();

        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[257]) };
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[257], v | (1 << 2)) };
        let mut d = crate::DEBUG_STUFF.lock();
        d[0] = unsafe { core::ptr::read_volatile(&internals.regs.regs[257]) };
        drop(d);
        drop(internals);
    }

    fn disable(&self) {
        let ltdc = self.ltdc.lock();
        ltdc.disable_clock();
        drop(ltdc);
        self.cc.disable_clock(4 * 32 + 27);
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

    /// is the regulator ready
    fn regulator_ready(&self) -> bool {
        let internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[259]) };
        (v & (1 << 12)) != 0
    }

    /// Set the dphy link speed
    fn set_dphy_link(&self, nv: u64) {
        let mut internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[262]) };
        let newval = (v & !0x3F) | (nv as u32 & 0x3F);
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[262], newval) };
    }
}

impl crate::modules::clock::ClockProviderTrait for Module {
    /// Enable the pll
    fn enable_clock(&self, _i: usize) {
        let mut internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) };
        let newval = v | 1;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[268], newval) };
    }

    /// Disable the pll
    fn disable_clock(&self, _i: usize) {
        let mut internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) };
        let newval = v & !1;
        unsafe { core::ptr::write_volatile(&mut internals.regs.regs[268], newval) };
    }

    fn clock_is_ready(&self, _i: usize) -> bool {
        let internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[259]) };
        (v & 1 << 8) != 0
    }

    fn clock_frequency(&self, i: usize) -> Option<u64> {
        if let Some(fin) = self.iclk[1].clock_frequency() {
            let id = self.get_input_divider();
            let vco_mul = self.get_multiplier();
            let divider = crate::modules::clock::PllTrait::get_post_divider(self, i) as u64;
            let fout = (2 * fin * vco_mul as u64) / (id as u64 * divider);
            return Some(fout);
        } else {
            return None;
        }
    }

    fn get_ref(&self, i: usize) -> crate::modules::clock::ClockRef {
        crate::modules::clock::ClockRef::Plain(crate::modules::clock::ClockRefPlain {
            clock_provider: self.clone().into(),
            index: i,
        })
    }
}

impl crate::modules::clock::PllTrait for Module {
    fn get_input_frequency(&self) -> Option<u64> {
        self.iclk[1].clock_frequency()
    }

    fn set_input_divider(&self, d: u32) -> Result<(), crate::modules::clock::PllDividerErr> {
        if (d & !7) != 0 {
            return Err(PllDividerErr::ImpossibleDivisor);
        }
        if let Some(fin) = self.iclk[1].clock_frequency() {
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
        if let Some(fin) = self.iclk[1].clock_frequency() {
            let vco_freq = fin as u32 * vco_mul as u32;
            let fout = vco_freq / (2 * id as u32 * d as u32);
            if !(31_250_000..=500_000_000).contains(&fout) {
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

        if let Some(fin) = self.iclk[1].clock_frequency() {
            let fin = fin / self.get_input_divider() as u64;
            let multiplier = f / (2 * fin);
            self.set_multiplier(multiplier as u32);
            Ok(())
        } else {
            Err(PllVcoSetError::UnknownInputFrequency)
        }
    }
}

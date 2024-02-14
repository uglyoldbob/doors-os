//! Dsi related code for the stm32f769

use alloc::vec::Vec;

use crate::modules::clock::{ClockProviderTrait, ClockRefTrait};
use crate::modules::clock::{PllDividerErr, PllVcoSetError};
use crate::modules::video::ScreenResolution;
use crate::LockedArc;

use super::DsiPanelTrait;

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

    fn write(&mut self, i: usize, val: u32) {
        use crate::modules::video::TextDisplayTrait;
        doors_macros2::kernel_print!("ltdc write register {:X} with {:X}\r\n", i * 4, val);
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[i], val) };
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
        self.write(6, 1);
        unsafe { core::ptr::read_volatile(&self.regs.regs[6]) };
    }

    /// Enable the clock input for the hardware
    pub fn enable_clock(&self) {
        self.cc.enable_clock(5 * 32 + 26);
    }

    /// Disable the clock input for the hardware
    pub fn disable_clock(&self) {
        self.cc.disable_clock(5 * 32 + 26);
    }

    pub fn configure(&mut self, resolution: &super::super::ScreenResolution) {
        let v = (resolution.hsync as u32 - 1) << 16 | (resolution.vsync as u32 - 1);
        self.write(2, v);

        let v = (resolution.h_b_porch as u32 + resolution.hsync as u32 - 1) << 16
            | (resolution.v_b_porch as u32 + resolution.vsync as u32 - 1);
        self.write(3, v);

        let v = (resolution.width as u32 + resolution.h_b_porch as u32 + resolution.hsync as u32
            - 1)
            << 16
            | (resolution.height as u32 + resolution.v_b_porch as u32 + resolution.vsync as u32
                - 1);
        self.write(4, v);

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
        self.write(5, v);

        let v = resolution.height as u32 + resolution.v_b_porch as u32 + resolution.vsync as u32;
        self.write(16, v);

        self.write(0x18 / 4, 0);

        self.write(11, 0xffffff);

        let v = unsafe { core::ptr::read_volatile(&self.regs.regs[3]) } >> 16;
        self.write(34, ((v + resolution.width as u32) << 16) | (v + 1));

        let v = unsafe { core::ptr::read_volatile(&self.regs.regs[3]) } & 0xFFFF;
        self.write(35, ((v + resolution.height as u32) << 16) | (v + 1));

        self.write(39, 0xffffffff);
        self.write(
            44,
            (resolution.width as u32 * 2) << 16 | (3 + resolution.width as u32 * 2),
        );
        self.write(37, 2);
        self.write(0x98 / 4, 0xff);

        self.write(40, 0x405);
        self.write(45, resolution.height as u32);
        self.write(43, 0x2002_0000);

        self.write(33, 1);

        //trigger immediate load
        self.write(9, 1);

        self.enable();
    }
}

/// The provider for dcs commands
pub struct DcsProvider {
    // The internals for the hardware
    internals: LockedArc<ModuleInternals>,
}

impl super::MipiDsiDcsTrait for DcsProvider {
    fn dcs_do_command<'a>(&mut self, cmd: &mut super::DcsCommand<'a>) -> Result<(), ()> {
        let flags = cmd.flags;
        let mut internals = self.internals.lock();
        let packet = cmd.build_packet();
        if packet.is_err() {
            return Err(());
        }
        let packet = packet.unwrap();
        internals.write_packet(&packet);
        drop(packet);
        if let Some(buf) = cmd.recv.as_mut() {
            internals.read_data(*buf);
        }
        Ok(())
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
    /// Write a register
    fn write(&mut self, i: usize, d: u32) {
        use crate::modules::video::TextDisplayTrait;
        doors_macros2::kernel_print!("Write {:X} with {:X}\r\n", i * 4, d);
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[i], d) };
    }

    /// Write the packet to the device registers
    fn write_packet(&mut self, packet: &super::DcsPacket) {
        use crate::modules::video::TextDisplayTrait;
        let buf = packet.data.unwrap_or(&[]);
        let mut length_remaining = buf.len();
        let h = u32::from_le_bytes(packet.header);
        doors_macros2::kernel_print!("dcs packet length {:x} {}\r\n", h, length_remaining);

        self.write(0x94 / 4, 0);
        self.write(0x68 / 4, 0x10f7f00);

        //Wait until command payload fifo are empty
        loop {
            let val = unsafe { core::ptr::read_volatile(&self.regs.regs[29]) };
            if (val & 1) == 1 {
                break;
            }
        }

        let mut offset = 0;
        while length_remaining > 0 {
            //Wait until write payload fifo not full
            loop {
                let val = unsafe { core::ptr::read_volatile(&self.regs.regs[29]) };
                if (val & (1 << 3)) == 0 {
                    break;
                }
            }

            if length_remaining < 4 {
                let mut tbuf: [u8; 4] = [0; 4];
                for i in 0..length_remaining {
                    tbuf[i] = buf[i + offset];
                }
                let contents = u32::from_le_bytes(tbuf);
                self.write(28, contents);
                length_remaining = 0;
            } else {
                let mut tbuf: [u8; 4] = [0; 4];
                for i in 0..4 {
                    tbuf[i] = buf[i + offset];
                }
                let contents = u32::from_le_bytes(tbuf);
                self.write(28, contents);
                offset += 4;
                length_remaining -= 4;
            }
        }

        //Write the header
        let header = u32::from_le_bytes(packet.header);

        //Wait until command fifo not full
        loop {
            let val = unsafe { core::ptr::read_volatile(&self.regs.regs[29]) };
            if (val & (1 << 1)) == 0 {
                break;
            }
        }
        self.write(27, header);

        //Wait until command payload and write fifo are empty
        loop {
            let val = unsafe { core::ptr::read_volatile(&self.regs.regs[29]) };
            if (val & 5) == 5 {
                break;
            }
        }
    }

    /// Read data from the bus
    fn read_data(&mut self, buf: &mut [u8]) {
        //Wait until read operation is complete
        loop {
            let val = unsafe { core::ptr::read_volatile(&self.regs.regs[29]) };
            if (val & (1 << 6)) == 0 {
                break;
            }
        }
        for i in (0..=buf.len()).step_by(4) {
            //wait until fifo not empty
            loop {
                let val = unsafe { core::ptr::read_volatile(&self.regs.regs[29]) };
                if (val & (1 << 4)) == 0 {
                    break;
                }
            }
            let val = unsafe { core::ptr::read_volatile(&self.regs.regs[28]) };
            let data = val.to_le_bytes();
            for j in 0..(buf.len() - i).min(4) {
                buf[i + j] = data[j];
            }
        }
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
    fn enable(&self, config: &super::MipiDsiConfig, panel: Option<super::DsiPanel>) {
        self.cc.enable_clock(5 * 32 + 27);
        let mut ltdc = self.ltdc.lock();
        ltdc.enable_clock();

        let mut resolutions: Vec<ScreenResolution> = Vec::new();
        if let Some(panel) = &panel {
            panel.get_resolutions(&mut resolutions);
        }
        let resolution = if resolutions.len() > 0 {
            Some(resolutions[0].clone())
        } else {
            None
        };

        //configure the pll
        let dsi_pll = crate::modules::clock::Pll::Stm32f769DsiPll(self.clone());
        loop {
            if crate::modules::clock::PllTrait::set_input_divider(&dsi_pll, 4).is_ok() {
                break;
            }
        }
        use crate::modules::video::TextDisplayTrait;
        doors_macros2::kernel_print!("setting dsi pll frequency\r\n");
        let e = crate::modules::clock::PllTrait::set_vco_frequency(&dsi_pll, 937_500_000);
        match e {
            Ok(_) => {}
            Err(e) => loop {
                match e {
                    PllVcoSetError::FrequencyOutOfRange => {
                        doors_macros2::kernel_print!("out of range\r\n")
                    }
                    PllVcoSetError::UnknownInputFrequency => {
                        doors_macros2::kernel_print!("unknown input frequency\r\n")
                    }
                    PllVcoSetError::CannotHitFrequency => {
                        doors_macros2::kernel_print!("cannot hit target frequency\r\n")
                    }
                    PllVcoSetError::InputFrequencyOutOfRange => {
                        doors_macros2::kernel_print!("input out of range\r\n")
                    }
                }
            },
        }
        doors_macros2::kernel_print!("setting dsi pll post divider\r\n");
        loop {
            if crate::modules::clock::PllTrait::set_post_divider(&dsi_pll, 0, 2).is_ok() {
                break;
            }
        }
        doors_macros2::kernel_print!("dsi pll freq is {:?}\r\n", dsi_pll.clock_frequency(0));

        let dsi_frequency = dsi_pll.clock_frequency(0).unwrap() / 2;
        let pixclock = dsi_frequency / 15;

        let val = 4_000_000_000 / config.link_speed;
        self.set_dphy_link(val);

        let mut internals = self.internals.lock();

        internals.write(0x400 / 4, 0);
        internals.write(0x400 / 4, 10);

        internals.write(1, 0);

        let rate = 20;
        //calculate the value of 3
        internals.write(2, 0xa00 | 3);

        internals.write(3, 0);
        internals.write(4, 5);
        internals.write(5, 0);
        internals.write(6, 0x40004);
        internals.write(0x2c / 4, 0x1c);
        internals.write(0x38 / 4, 0x3f02);
        if let Some(resolution) = &resolution {
            internals.write(0x3c / 4, resolution.width as u32);
        }
        internals.write(0x78 / 4, 1000 << 16 | 1000);
        internals.write(0x8c / 4, 0xd00);
        internals.write(0x34 / 4, 1);

        internals.write(0x404 / 4, 0);

        if let Some(resolution) = &resolution {
            let htotal =
                (resolution.h_b_porch + resolution.h_f_porch + resolution.width + resolution.hsync)
                    as u64;
            let calc1 = htotal * dsi_frequency / 8000;
            let (f, mut c2) = (calc1 % (pixclock / 1000), calc1 / (pixclock / 1000));
            if f != 0 {
                c2 += 1;
            }
            c2 = 0x4fb; //todo remove this hack
            internals.write(0x50 / 4, c2 as u32);

            let hsa = resolution.hsync as u64;
            let calc1: u64 = hsa * dsi_frequency / 8000;
            let (f, mut c2) = (calc1 % (pixclock / 1000), calc1 / (pixclock / 1000));
            if f != 0 {
                c2 += 1;
            }
            c2 = 0x3a; //todo remove this hack
            internals.write(0x48 / 4, c2 as u32);

            let hbp = resolution.h_b_porch as u64;
            let calc1: u64 = hbp * dsi_frequency / 8000;
            let (f, mut c2) = (calc1 % (pixclock / 1000), calc1 / (pixclock / 1000));
            if f != 0 {
                c2 += 1;
            }
            c2 = 0xb1; //todo remove this hack
            internals.write(0x4c / 4, c2 as u32);

            internals.write(0x60 / 4, resolution.height as u32);
            internals.write(0x54 / 4, resolution.vsync as u32);
            internals.write(0x5c / 4, resolution.v_f_porch as u32);
            internals.write(0x58 / 4, resolution.v_b_porch as u32);
        }

        internals.write(0xa0 / 4, 0);
        internals.write(0xb4 / 4, 0);
        internals.write(0xb4 / 4, 1);
        internals.write(0xb4 / 4, 0);

        //todo Calculate these
        internals.write(0x9c / 4, 0x40402710);
        internals.write(0x98 / 4, 0x400040);

        let nlanes = 1;
        internals.write(0xa4 / 4, 0x2000 | nlanes);

        unsafe { core::ptr::read_volatile(&internals.regs.regs[0xbc / 4]) };
        unsafe { core::ptr::read_volatile(&internals.regs.regs[0xc0 / 4]) };
        internals.write(0xc4 / 4, 0);
        internals.write(0xc8 / 4, 0);

        drop(internals);
        //enable and wait for the pll
        self.enable_regulator();
        loop {
            if self.regulator_ready() {
                break;
            }
        }
        let pll_provider = crate::modules::clock::ClockProvider::Stm32f769DsiPll(self.clone());
        crate::modules::clock::ClockProvider::enable_clock(&pll_provider, 0);
        while !crate::modules::clock::ClockProvider::clock_is_ready(&pll_provider, 0) {}

        let mut internals = self.internals.lock();

        internals.write(0xa0 / 4, 0xf);

        loop {
            let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[0xb0 / 4]) };
            if (v & 4) != 0 {
                break;
            }
        }

        {
            use crate::modules::timer::TimerTrait;
            let mut timers = crate::kernel::TIMERS.lock();
            let tp = timers.module(0);
            drop(timers);
            let mut tpl = tp.lock();
            let timer = tpl.get_timer(0).unwrap();
            drop(tpl);
            crate::modules::timer::TimerInstanceTrait::delay_ms(&timer, 40);
        }

        internals.write(1, 0);
        internals.write(0x34 / 4, 1);
        internals.write(0x404 / 4, 0);
        internals.write(1, 1);

        drop(internals);

        if let Some(resolution) = &resolution {
            doors_macros2::kernel_print!("Setting up ltdc hardware with screen resolution\r\n");
            ltdc.configure(resolution);
        }

        if let Some(panel) = panel {
            panel.setup(&mut super::MipiDsiDcs::Stm32f769(DcsProvider {
                internals: self.internals.clone(),
            }));
        }
        let mut internals = self.internals.lock();

        internals.write(1, 0);
        internals.write(0x34 / 4, 0);
        internals.write(0x38 / 4, 0x3f02);
        internals.write(0x94 / 4, 1);

        //enable dsi wrapper
        internals.write(0x404 / 4, 8);

        internals.write(1, 1);

        internals.write(0x38 / 4, 0x01013f02);

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
        internals.write(268, newval);
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
        internals.write(268, newval);
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
        internals.write(262, newval);
    }
}

impl crate::modules::clock::ClockProviderTrait for Module {
    /// Enable the pll
    fn enable_clock(&self, _i: usize) {
        let mut internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) };
        let newval = v | 1;
        internals.write(268, newval);
    }

    /// Disable the pll
    fn disable_clock(&self, _i: usize) {
        let mut internals = self.internals.lock();
        let v = unsafe { core::ptr::read_volatile(&internals.regs.regs[268]) };
        let newval = v & !1;
        internals.write(268, newval);
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
        internals.write(268, newval);
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
        internals.write(268, newval);
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

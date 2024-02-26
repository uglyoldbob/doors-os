//! The memory controllers for the stm32f769 processor.

use alloc::vec::Vec;

use crate::modules::gpio::GpioPinTrait;

/// The registers for the peripheral.
pub struct Registers {
    regs: [u32; 10],
}

/// The flash memory controller for the stm32f769
pub struct FlashInterfaceController {
    /// The memory mapped register set for the hardware
    regs: &'static mut Registers,
}

impl FlashInterfaceController {
    /// Create a new object
    pub unsafe fn new(addr: usize) -> Self {
        Self {
            regs: &mut *(addr as *mut Registers),
        }
    }

    /// Set the number of wait states for the memory controller
    pub fn set_wait_states(&mut self, v: u8) {
        let d = unsafe { core::ptr::read_volatile(&self.regs.regs[0]) } & !0xF;
        unsafe { core::ptr::write_volatile(&mut self.regs.regs[0], d | (v as u32 & 0xF)) };
        unsafe { core::ptr::read_volatile(&self.regs.regs[0]) };
    }
}

/// The controller for sdram on the stm32f769
pub struct SdramController {
    /// The registers
    regs: &'static mut [u32],
    /// The input clock
    clock: crate::modules::clock::ClockRef,
    /// The pins for the controller
    pins: Vec<crate::modules::gpio::GpioPin>,
}

impl SdramController {
    /// Create a new object
    pub unsafe fn new(addr: usize, clock: crate::modules::clock::ClockRef) -> Self {
        Self {
            regs: core::slice::from_raw_parts_mut(addr as *mut u32, 87),
            clock,
            pins: Vec::new(),
        }
    }

    /// Wait until the sdram controller is not busy
    fn sdram_wait_until_not_busy(&mut self) {
        loop {
            let val = unsafe { core::ptr::read_volatile(&self.regs[0x158 / 4]) };
            if (val & (1 << 5)) == 0 {
                break;
            }
        }
    }

    /// Setup sdram memory. TODO handle the second bank
    pub fn setup_sdram(&mut self) -> &'static mut [u8] {
        use crate::modules::clock::ClockRefTrait;
        use crate::modules::gpio::GpioTrait;
        self.clock.enable_clock();

        let mut gpio = crate::kernel::GPIO.lock();
        let gd = gpio.module(3);
        let ge = gpio.module(4);
        let gf = gpio.module(5);
        let gg = gpio.module(6);
        let gh = gpio.module(7);
        let gi = gpio.module(8);
        let gpiod = gd.lock();
        let gpioe = ge.lock();
        let gpiof = gf.lock();
        let gpiog = gg.lock();
        let gpioh = gh.lock();
        let gpioi = gi.lock();

        //setup the fmc pins for sdram

        let mut pins = [
            gpiod.get_pin(14).unwrap(), //d0
            gpiod.get_pin(14).unwrap(), //d0
            gpiod.get_pin(15).unwrap(), //d1
            gpiod.get_pin(0).unwrap(),  //d2
            gpiod.get_pin(1).unwrap(),  //d3
            gpioe.get_pin(7).unwrap(),  //d4
            gpioe.get_pin(8).unwrap(),  //d5
            gpioe.get_pin(9).unwrap(),  //d6
            gpioe.get_pin(10).unwrap(), //d7
            gpioe.get_pin(11).unwrap(), //d8
            gpioe.get_pin(12).unwrap(), //d9
            gpioe.get_pin(13).unwrap(), //d10
            gpioe.get_pin(14).unwrap(), //d11
            gpioe.get_pin(15).unwrap(), //d12
            gpiod.get_pin(8).unwrap(),  //d13
            gpiod.get_pin(9).unwrap(),  //d14
            gpiod.get_pin(10).unwrap(), //d15
            gpioh.get_pin(8).unwrap(),  //d16
            gpioh.get_pin(9).unwrap(),  //d17
            gpioh.get_pin(10).unwrap(), //d18
            gpioh.get_pin(11).unwrap(), //d19
            gpioh.get_pin(12).unwrap(), //d20
            gpioh.get_pin(13).unwrap(), //d21
            gpioh.get_pin(14).unwrap(), //d22
            gpioh.get_pin(15).unwrap(), //d23
            gpioi.get_pin(0).unwrap(),  //d24
            gpioi.get_pin(1).unwrap(),  //d25
            gpioi.get_pin(2).unwrap(),  //d26
            gpioi.get_pin(3).unwrap(),  //d27
            gpioi.get_pin(6).unwrap(),  //d28
            gpioi.get_pin(7).unwrap(),  //d29
            gpioi.get_pin(9).unwrap(),  //d30
            gpioi.get_pin(10).unwrap(), //d31
            gpiof.get_pin(0).unwrap(),  //a0
            gpiof.get_pin(1).unwrap(),  //a1
            gpiof.get_pin(2).unwrap(),  //a2
            gpiof.get_pin(3).unwrap(),  //a3
            gpiof.get_pin(4).unwrap(),  //a4
            gpiof.get_pin(5).unwrap(),  //a5
            gpiof.get_pin(12).unwrap(), //a6
            gpiof.get_pin(13).unwrap(), //a7
            gpiof.get_pin(14).unwrap(), //a8
            gpiof.get_pin(15).unwrap(), //a9
            gpiog.get_pin(0).unwrap(),  //a10
            gpiog.get_pin(1).unwrap(),  //a11
            //a12 not connected
            gpiog.get_pin(4).unwrap(),  //ba0
            gpiog.get_pin(5).unwrap(),  //ba1
            gpioe.get_pin(0).unwrap(),  //nbl0
            gpioe.get_pin(1).unwrap(),  //nbl1
            gpioi.get_pin(4).unwrap(),  //nbl2
            gpioi.get_pin(5).unwrap(),  //nbl3
            gpiog.get_pin(8).unwrap(),  //clk
            gpioh.get_pin(2).unwrap(),  //cke0
            gpioh.get_pin(3).unwrap(),  //ne0
            gpiof.get_pin(11).unwrap(), //nras
            gpiog.get_pin(15).unwrap(), //ncas
            gpioh.get_pin(5).unwrap(),  //nwe
        ];

        for p in pins.iter_mut() {
            p.set_alternate(12);
            p.set_speed(3);
        }

        for p in pins {
            self.pins.push(p);
        }

        let control = SdramControl {
            columns: 0,
            rows: 1,
            width: 2,
            banks: 1,
            cas_latency: 3,
            sdclk: 2,
            burst: 1,
            pipe_delay: 0,
        };

        let timing = SdramTiming {
            tmrd: 1,
            txsr: 5,
            tras: 3,
            trc: 5,
            twr: 1,
            trp: 1,
            trcd: 1,
        };

        let val = (control.columns as u32)
            | ((control.rows as u32) << 2)
            | ((control.width as u32) << 4)
            | ((control.banks as u32) << 6)
            | ((control.cas_latency as u32) << 7)
            | ((control.sdclk as u32) << 10)
            | ((control.burst as u32) << 12)
            | ((control.pipe_delay as u32) << 13);
        unsafe { core::ptr::write_volatile(&mut self.regs[(0x140) / 4], val) };

        let val = (timing.tmrd as u32)
            | (timing.txsr as u32) << 4
            | (timing.tras as u32) << 8
            | (timing.trc as u32) << 12
            | (timing.twr as u32) << 16
            | (timing.trp as u32) << 20
            | (timing.trcd as u32) << 24;
        unsafe { core::ptr::write_volatile(&mut self.regs[(0x148) / 4], val) };

        // clock configuration enable for bank 1
        let val = (1 << 4) | 1;
        unsafe { core::ptr::write_volatile(&mut self.regs[(0x150) / 4], val) };

        {
            use crate::modules::timer::TimerTrait;
            let mut timers = crate::kernel::TIMERS.lock();
            let tp = timers.module(0);
            drop(timers);
            let mut tpl = tp.lock();
            let timer = tpl.get_timer(0).unwrap();
            drop(tpl);
            crate::modules::timer::TimerInstanceTrait::delay_us(&timer, 200);
        }
        self.sdram_wait_until_not_busy();

        //precharge bank
        let val = (1 << 4) | 2;
        unsafe { core::ptr::write_volatile(&mut self.regs[(0x150) / 4], val) };
        {
            use crate::modules::timer::TimerTrait;
            let mut timers = crate::kernel::TIMERS.lock();
            let tp = timers.module(0);
            drop(timers);
            let mut tpl = tp.lock();
            let timer = tpl.get_timer(0).unwrap();
            drop(tpl);
            crate::modules::timer::TimerInstanceTrait::delay_us(&timer, 100);
        }
        self.sdram_wait_until_not_busy();

        // auto-refresh command with 8 auto-refresh cycles
        let val = (1 << 4) | 3 | 7 << 5;
        unsafe { core::ptr::write_volatile(&mut self.regs[(0x150) / 4], val) };
        {
            use crate::modules::timer::TimerTrait;
            let mut timers = crate::kernel::TIMERS.lock();
            let tp = timers.module(0);
            drop(timers);
            let mut tpl = tp.lock();
            let timer = tpl.get_timer(0).unwrap();
            drop(tpl);
            crate::modules::timer::TimerInstanceTrait::delay_us(&timer, 100);
        }
        self.sdram_wait_until_not_busy();

        //load mode register with cas latency
        let val = (1 << 4) | 4 | (control.cas_latency as u32) << 13;
        unsafe { core::ptr::write_volatile(&mut self.regs[(0x150) / 4], val) };
        {
            use crate::modules::timer::TimerTrait;
            let mut timers = crate::kernel::TIMERS.lock();
            let tp = timers.module(0);
            drop(timers);
            let mut tpl = tp.lock();
            let timer = tpl.get_timer(0).unwrap();
            drop(tpl);
            crate::modules::timer::TimerInstanceTrait::delay_us(&timer, 100);
        }
        self.sdram_wait_until_not_busy();

        //normal mode command
        let val = 1 << 4;
        unsafe { core::ptr::write_volatile(&mut self.regs[(0x150) / 4], val) };

        //refresh the timer
        let val = 1542 << 1;
        unsafe { core::ptr::write_volatile(&mut self.regs[(0x154) / 4], val) };

        unsafe { core::slice::from_raw_parts_mut(0xc000_0000 as *mut u8, 16 * 1024 * 1024) }
    }
}

/// Data specific to the stm32 flash memory controller sdram control register
struct SdramControl {
    /// Number of bits of column address. 0 = 8 bits, 1 = 9 bits, 2 = 10 bits, 3 = 11 bits.
    columns: u8,
    /// Number of bits of row address. 0 = 11 bits, 1 = 12 bits, 2 = 13 bits, 3 = reserved.
    rows: u8,
    /// Memory device width. 0 = 8 bits, 1 = 16 bits, 2 = 32 bits, 3 = reserved.
    width: u8,
    /// Number of internal banks. 0 = 2 banks, 1 = four banks.
    banks: u8,
    /// sdram cas latency in clock cycles. 1 - 3, 0 is reserved.
    cas_latency: u8,
    /// sets the sdram clock period. 0 = disabled, 1 = reserved, 2 = 2xHCLK, 3 = 3xHCLK.
    sdclk: u8,
    /// 1 enables burst read mode
    burst: u8,
    /// Number of HCLK cycles of delay for reading after CAS latency. 0 - 2, 3 is reserved.
    pipe_delay: u8,
}

/// Data specific to the stm32 flash memory controller sdram timing register
struct SdramTiming {
    /// Delay between load mode register command and active or refresh command in memory clock cycles. 0 = 1 cycle, 15 = 16 cycles.
    tmrd: u8,
    /// delay in memory clock cycles from release self-refresh to issuing the activate command. 0 = 1 cycle, 15 = 16 cycles.
    txsr: u8,
    /// Minimum number of cycles for self-refresh period. 0 = 1 cycle, 15 = 16 cycles.
    tras: u8,
    /// Delay between refresh and activate command, as well as between two consecutive refreshes. Both banks share this and the slowest one must be used. 0 = 1 cycle, 15 = 16 cycles.
    trc: u8,
    /// Delay between write and precharge command. 0 = 1 cycle, 15 = 16 cycles.
    twr: u8,
    /// Delay between precharge and another command. Both banks share this and the slowest one must be used. 0 = 1 cycle, 15 = 16 cycles.
    trp: u8,
    /// Delay between active and read/write commands. 0 = 1 cycle, 15 = 16 cycles.
    trcd: u8,
}

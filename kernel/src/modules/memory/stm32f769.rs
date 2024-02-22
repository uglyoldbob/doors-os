//! The memory controllers for the stm32f769 processor.

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
}

impl SdramController {
    /// Create a new object
    pub unsafe fn new(addr: usize, clock: crate::modules::clock::ClockRef) -> Self {
        Self {
            regs: core::slice::from_raw_parts_mut(addr as *mut u32, 87),
            clock,
        }
    }

    /// Wait until the sdram controller is not busy
    fn sdram_wait_until_not_busy(&mut self) {
        loop {
            let val = unsafe { core::ptr::read_volatile(&self.regs[0x158/4]) };
            if (val & (1<<5)) == 0 {
                break;
            }
        }
    }

    /// Setup sdram memory. TODO handle the second bank
    pub fn setup_sdram(&mut self) -> &'static mut [u8] {
        use crate::modules::clock::ClockRefTrait;
        use crate::modules::gpio::GpioTrait;
        self.clock.enable_clock();

        {
            let mut gpio = crate::kernel::GPIO.lock();
            let gd = gpio.module(3);
            let ge = gpio.module(4);
            let gf = gpio.module(5);
            let gg = gpio.module(6);
            let gh = gpio.module(7);
            let gi = gpio.module(8);
            let mut gpiod = gd.lock();
            let mut gpioe = ge.lock();
            let mut gpiof = gf.lock();
            let mut gpiog = gg.lock();
            let mut gpioh = gh.lock();
            let mut gpioi = gi.lock();

            gpiod.reset(false);
            gpioe.reset(false);
            gpiof.reset(false);
            gpiog.reset(false);
            gpioh.reset(false);
            gpioi.reset(false);
            //setup the fmc pins for sdram

            gpiod.set_alternate(14, 12); //d0
            gpiod.set_alternate(15, 12); //d1
            gpiod.set_alternate(0, 12); //d2
            gpiod.set_alternate(1, 12); //d3
            gpioe.set_alternate(7, 12); //d4
            gpioe.set_alternate(8, 12); //d5
            gpioe.set_alternate(9, 12); //d6
            gpioe.set_alternate(10, 12); //d7
            gpioe.set_alternate(11, 12); //d8
            gpioe.set_alternate(12, 12); //d9
            gpioe.set_alternate(13, 12); //d10
            gpioe.set_alternate(14, 12); //d11
            gpioe.set_alternate(15, 12); //d12
            gpiod.set_alternate(8, 12); //d13
            gpiod.set_alternate(9, 12); //d14
            gpiod.set_alternate(10, 12); //d15
            gpioh.set_alternate(8, 12); //d16
            gpioh.set_alternate(9, 12); //d17
            gpioh.set_alternate(10, 12); //d18
            gpioh.set_alternate(11, 12); //d19
            gpioh.set_alternate(12, 12); //d20
            gpioh.set_alternate(13, 12); //d21
            gpioh.set_alternate(14, 12); //d22
            gpioh.set_alternate(15, 12); //d23
            gpioi.set_alternate(0, 12); //d24
            gpioi.set_alternate(1, 12); //d25
            gpioi.set_alternate(2, 12); //d26
            gpioi.set_alternate(3, 12); //d27
            gpioi.set_alternate(6, 12); //d28
            gpioi.set_alternate(7, 12); //d29
            gpioi.set_alternate(9, 12); //d30
            gpioi.set_alternate(10, 12); //d31

            gpiof.set_alternate(0, 12); //a0
            gpiof.set_alternate(1, 12); //a1
            gpiof.set_alternate(2, 12); //a2
            gpiof.set_alternate(3, 12); //a3
            gpiof.set_alternate(4, 12); //a4
            gpiof.set_alternate(5, 12); //a5
            gpiof.set_alternate(12, 12); //a6
            gpiof.set_alternate(13, 12); //a7
            gpiof.set_alternate(14, 12); //a8
            gpiof.set_alternate(15, 12); //a9
            gpiog.set_alternate(0, 12); //a10
            gpiog.set_alternate(1, 12); //a11
            //a12 not connected

            gpiog.set_alternate(4, 12); //ba0
            gpiog.set_alternate(5, 12); //ba1

            gpioe.set_alternate(0, 12); //nbl0
            gpioe.set_alternate(1, 12); //nbl1
            gpioi.set_alternate(4, 12); //nbl2
            gpioi.set_alternate(5, 12); //nbl3

            gpiog.set_alternate(8, 12); //clk
            gpioh.set_alternate(2, 12); //cke0
            gpioh.set_alternate(3, 12); //ne0
            gpiof.set_alternate(11, 12); //nras
            gpiog.set_alternate(15, 12); //ncas
            gpioh.set_alternate(5, 12); //nwe

            //set gpio speed for maximum
            gpiod.set_speed(14, 3); //d0
            gpiod.set_speed(15, 3); //d1
            gpiod.set_speed(0, 3); //d2
            gpiod.set_speed(1, 3); //d3
            gpioe.set_speed(7, 3); //d4
            gpioe.set_speed(8, 3); //d5
            gpioe.set_speed(9, 3); //d6
            gpioe.set_speed(10, 3); //d7
            gpioe.set_speed(11, 3); //d8
            gpioe.set_speed(12, 3); //d9
            gpioe.set_speed(13, 3); //d10
            gpioe.set_speed(14, 3); //d11
            gpioe.set_speed(15, 3); //d12
            gpiod.set_speed(8, 3); //d13
            gpiod.set_speed(9, 3); //d14
            gpiod.set_speed(10, 3); //d15
            gpioh.set_speed(8, 3); //d16
            gpioh.set_speed(9, 3); //d17
            gpioh.set_speed(10, 3); //d18
            gpioh.set_speed(11, 3); //d19
            gpioh.set_speed(12, 3); //d20
            gpioh.set_speed(13, 3); //d21
            gpioh.set_speed(14, 3); //d22
            gpioh.set_speed(15, 3); //d23
            gpioi.set_speed(0, 3); //d24
            gpioi.set_speed(1, 3); //d25
            gpioi.set_speed(2, 3); //d26
            gpioi.set_speed(3, 3); //d27
            gpioi.set_speed(6, 3); //d28
            gpioi.set_speed(7, 3); //d29
            gpioi.set_speed(9, 3); //d30
            gpioi.set_speed(10, 3); //d31

            gpiof.set_speed(0, 3); //a0
            gpiof.set_speed(1, 3); //a1
            gpiof.set_speed(2, 3); //a2
            gpiof.set_speed(3, 3); //a3
            gpiof.set_speed(4, 3); //a4
            gpiof.set_speed(5, 3); //a5
            gpiof.set_speed(12, 3); //a6
            gpiof.set_speed(13, 3); //a7
            gpiof.set_speed(14, 3); //a8
            gpiof.set_speed(15, 3); //a9
            gpiog.set_speed(0, 3); //a10
            gpiog.set_speed(1, 3); //a11
            //a12 not connected

            gpiog.set_speed(4, 3); //ba0
            gpiog.set_speed(5, 3); //ba1

            gpioe.set_speed(0, 3); //nbl0
            gpioe.set_speed(1, 3); //nbl1
            gpioi.set_speed(4, 3); //nbl2
            gpioi.set_speed(5, 3); //nbl3

            gpiog.set_speed(8, 3); //clk
            gpioh.set_speed(2, 3); //cke0
            gpioh.set_speed(3, 3); //ne0
            gpiof.set_speed(11, 3); //nras
            gpiog.set_speed(15, 3); //ncas
            gpioh.set_speed(5, 3); //nwe

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
            | (timing.txsr as u32)<<4
            | (timing.tras as u32)<<8
            | (timing.trc as u32)<<12
            | (timing.twr as u32)<<16
            | (timing.trp as u32)<<20
            | (timing.trcd as u32)<<24;
        unsafe { core::ptr::write_volatile(&mut self.regs[(0x148) / 4], val) };
        
        // clock configuration enable for bank 1
        let val = (1<<4) | 1;
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
        let val = (1<<4) | 2;
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
        let val = (1<<4) | 3 | 7<<5;
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
        let val = (1<<4) | 4 | (control.cas_latency as u32)<<13;
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
        let val = 1<<4;
        unsafe { core::ptr::write_volatile(&mut self.regs[(0x150) / 4], val) };

        //refresh the timer
        let val = 1542<<1;
        unsafe { core::ptr::write_volatile(&mut self.regs[(0x154) / 4], val) };

        unsafe { core::slice::from_raw_parts_mut(0xc000_0000 as *mut u8, 32 * 1024 * 1024) }
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


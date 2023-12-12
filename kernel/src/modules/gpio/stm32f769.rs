//! The driver for the gpio on the the stm32f769 processor

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

/// A single stm32f769 gpio module
pub struct Gpio<'a> {
    /// the memory mapped registers for the hardware
    registers: &'a mut GpioRegisters,
}

impl<'a> Gpio<'a> {
    /// Construct a new gpio module with the specified address.
    pub unsafe fn new(addr: u32) -> Self {
        Self {
            registers: &mut *(addr as *mut GpioRegisters),
        }
    }
}

impl<'a> super::GpioTrait for Gpio<'a> {
    #[doc = " A test function to do something"]
    fn do_something(&mut self) {
        todo!()
    }

    fn set_output(&mut self, i: usize) {
        assert!(i < 16);
        let mode_filter = (3u32) << (2 * i);
        let nm = self.registers.mode & !mode_filter;
        let mode = (1u32) << (2 * i);
        self.registers.mode = nm | mode;
        let n = 4u32;
        if i < 16 {
            let mask = 15u32 << (i * 4);
            let newf = n << (i * 4);
            let newval = (self.registers.afrl & !mask) | newf;
            self.registers.afrl = newval;
        } else {
            let ri = i & 15;
            let mask = 15u32 << (ri * 4);
            let newf = n << (ri * 4);
            self.registers.afrh = (self.registers.afrh & !mask) | newf;
        }
    }

    fn write_output(&mut self, i: usize, v: bool) {
        assert!(i < 16);
        let m = 1 << i;
        let newval = if v {
            (self.registers.odr & !m) | m
        } else {
            (self.registers.odr & !m)
        };
        while self.registers.odr != newval {
            self.registers.odr = newval;
        }
    }
}

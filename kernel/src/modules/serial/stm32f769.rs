//! Serial code for the stm32f769

use crate::LockedArc;

/// The register set for the usart hardware
struct UsartRegisters {
    regs: [u32; 11],
}

impl UsartRegisters {
    fn tx_empty(&self) -> bool {
        let v = unsafe { core::ptr::read_volatile(&self.regs[7]) };
        (v & 1 << 7) != 0
    }

    fn enable_tx(&mut self) {
        let v = unsafe { core::ptr::read_volatile(&self.regs[0]) };
        unsafe { core::ptr::write_volatile(&mut self.regs[0], v | 8) };
    }

    fn disable_tx(&mut self) {
        let v = unsafe { core::ptr::read_volatile(&self.regs[0]) };
        unsafe { core::ptr::write_volatile(&mut self.regs[0], v & !8) };
    }
}

/// A serial port on the stm32f769
pub struct Usart {
    /// The registers for the hardware
    regs: &'static mut UsartRegisters,
    /// The clock line for this hardware
    clock: crate::modules::clock::ClockRef,
    /// The pins for the serial port, so they can be pinmuxed
    pins: [Option<crate::modules::gpio::GpioPin>; 2],
}

impl Usart {
    /// Create an instance of the usart hardware
    pub unsafe fn new(
        addr: u32,
        c: crate::modules::clock::ClockRef,
        gpios: [Option<crate::modules::gpio::GpioPin>; 2],
    ) -> Self {
        Self {
            regs: &mut *(addr as *mut UsartRegisters),
            clock: c,
            pins: gpios,
        }
    }
}

impl super::SerialTrait for LockedArc<Usart> {
    fn setup(&self, rate: u32) -> Result<(), ()> {
        use crate::modules::clock::ClockRefTrait;
        let mut s = self.lock();
        for up in &mut s.pins {
            if let Some(p) = up {
                use crate::modules::gpio::GpioPinTrait;
                p.set_alternate(7);
            }
        }
        s.clock.enable_clock();

        if let Some(ifreq) = s.clock.clock_frequency() {
            let divider = ifreq / rate as u64;
            unsafe { core::ptr::write_volatile(&mut s.regs.regs[3], (divider as u32) & 0xffff) };
            //9 bit word length means 8 bits + 1 optional parity
            unsafe { core::ptr::write_volatile(&mut s.regs.regs[0], 0x00001000) };

            //enable hardware
            unsafe { core::ptr::write_volatile(&mut s.regs.regs[0], 0x00001001) };
            Ok(())
        } else {
            Err(())
        }
    }

    fn sync_transmit(&self, data: &[u8]) {
        let mut s = self.lock();

        s.regs.enable_tx();
        while !s.regs.tx_empty() {}
        for b in data {
            unsafe { core::ptr::write_volatile(&mut s.regs.regs[10], *b as u32) };
            while !s.regs.tx_empty() {}
        }
        s.regs.disable_tx();
    }

    fn sync_transmit_str(&self, data: &str) {
        let mut s = self.lock();

        s.regs.enable_tx();
        while !s.regs.tx_empty() {}
        for b in data.bytes() {
            unsafe { core::ptr::write_volatile(&mut s.regs.regs[10], b as u32) };
            while !s.regs.tx_empty() {}
        }
        s.regs.disable_tx();
    }
}

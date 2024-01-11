//! Serial code for the stm32f769

use crate::LockedArc;

/// The register set for the usart hardware
struct UsartRegisters {
    regs: [u32; 11],
}

/// A serial port on the stm32f769
pub struct Usart {
    /// The registers for the hardware
    regs: &'static mut UsartRegisters,
    /// The clock line for this hardware
    clock: crate::modules::clock::ClockRef,
}

impl Usart {
    /// Create an instance of the usart hardware
    pub unsafe fn new(addr: u32, c: crate::modules::clock::ClockRef) -> Self {
        Self {
            regs: &mut *(addr as *mut UsartRegisters),
            clock: c,
        }
    }
}

impl super::SerialTrait for LockedArc<Usart> {
    fn setup(&self, rate: u32) -> Result<(), ()> {
        use crate::modules::clock::ClockRefTrait;
        let mut s = self.lock();
        s.clock.enable_clock();

        if let Some(ifreq) = s.clock.clock_frequency() {
            let divider = ifreq / rate as u64;
            let mut debug = crate::DEBUG_STUFF.lock();
            debug[0] = ifreq as u32;
            debug[1] = rate;
            debug[2] = divider as u32;
            debug[3] = 42;
            unsafe { core::ptr::write_volatile(&mut s.regs.regs[3], (divider as u32) & 0xffff) };
            Ok(())
        } else {
            Err(())
        }
    }
}

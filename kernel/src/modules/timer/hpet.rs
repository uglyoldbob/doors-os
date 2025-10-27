//! For the hpet (high performance timer) - normally found in x86 platforms

use alloc::{boxed::Box, vec::Vec};
use core::marker::PhantomData;

use crate::{kernel::SystemTrait, Arc, IrqGuarded, IrqGuardedInner};

#[repr(C)]
struct HpetChannelRegisters {
    /// configuration and capabilities of the channel
    config: u64,
    /// comparator value
    comparator: u64,
    /// interrupt routing
    interrupt: u64,
    /// dummy
    dummy: u64,
}

/// Verifies that a PageTable is the correct size
const _HPET_CHANNEL_SIZE_CHECKER: [u8; 0x20] = [0; core::mem::size_of::<HpetChannelRegisters>()];

/// The register for the hpet timer
#[repr(C)]
struct HpetRegisters {
    /// general capabilities and id
    general: u64,
    /// reserved space
    reserved1: u64,
    /// general configuration
    config: u64,
    /// reserved space
    reserved2: u64,
    /// interrupt status
    interrupt: u64,
    /// reserved space
    reserved3: u64,
    /// reserved space
    reserved4: [u64; 24],
    /// main counter value
    counter: u64,
    /// reserved space
    reserved5: u64,
    /// The individual channels
    channels: [HpetChannelRegisters; 32],
}

/// Verifies that a PageTable is the correct size
const _HPET_SIZE_CHECKER: [u8; 0x100 + 0x20 * 32] = [0; core::mem::size_of::<HpetRegisters>()];

struct HpetInternal {
    registers: crate::IrqGuarded<&'static mut HpetRegisters>,
    /// irq numbers
    irqs: [u8; 32],
    /// The number of channels present for the hpet
    num_channels: u8,
    /// Period in femtoseconds
    period: u32,
}

/// The main struct for the hpet implementation
pub struct Hpet {
    internal: Arc<HpetInternal>,
    irqs: Vec<u8>,
}

impl Hpet {
    /// Construct a new timer module
    pub fn new(addr: usize, num_channels: u8) -> Self {
        let r = unsafe { &mut *(addr as *mut HpetRegisters) };
        let mut irqs = Vec::with_capacity(32);
        let mut irq_values = [0; 32];
        let period = (r.general >> 32) as u32;
        for i in 0..num_channels {
            let creg = &mut r.channels[i as usize];
            let rcap = (creg.config >> 32) as u32;
            crate::VGA.print_str(&alloc::format!("HPET CHANNEL {} CONFIG {:x}\r\n", i, rcap));
            for index in 1..24 {
                if crate::SYSTEM.read().register_irq_handler(index, || {}) {
                    if !irqs.contains(&index) {
                        irqs.push(index);
                    }
                    unsafe {
                        crate::SYSTEM.read().unregister_irq_handler(index);
                    }
                    if ((rcap >> index) & 1) != 0 {
                        crate::VGA.print_str(&alloc::format!(
                            "HPET CHANNEL {} USE IRQ {}\r\n",
                            i,
                            index
                        ));
                        irq_values[i as usize] = index;
                        break;
                    }
                }
            }
        }
        let irqnums = crate::IrqNumbers::Many(irqs.clone());
        let com = IrqGuardedInner::new(irqnums, false, true, |_| {}, |_| {});
        loop {
            r.config = r.config | 1;
            crate::VGA.print_str(&alloc::format!("HPET CONFIG {:x}\r\n", r.config));
            if (r.config & 1) != 0 {
                break;
            }
        }
        crate::VGA.print_str(&alloc::format!("HPET PERIOD IS {}\r\n", period));
        let s = HpetInternal {
            registers: IrqGuarded::new(r, &com),
            irqs: irq_values,
            num_channels,
            period,
        };
        let s = Self {
            internal: Arc::new(s),
            irqs,
        };
        s
    }

    /// Test the hpet functionality
    pub fn test(&self) {
        let sys = crate::SYSTEM.read();
        for i in &self.irqs {
            let s2 = self.internal.clone();
            sys.register_irq_handler(*i, move || Self::handle_interrupt(&s2));
            sys.enable_irq(*i);
            crate::VGA.print_str(&alloc::format!("Enabled hpet irq {}\r\n", i));
        }
        let t = self.internal.registers.sync_access().counter;
        crate::VGA.print_str(&alloc::format!("HPET COUNT AT {}\r\n", t));
        let mut a = 0;
        let mut b;
        loop {
            let s = self.internal.registers.sync_access();
            b = s.counter;
            if b != t {
                break;
            }
            a += 1;
        }
        crate::VGA.print_str(&alloc::format!(
            "HPET COUNT AT {} took {} iterations to hit {}\r\n",
            t,
            a,
            b
        ));
    }

    fn handle_interrupt(s: &Arc<HpetInternal>) {
        loop {}
    }
}

/// A single timer channel for the hpet timer
pub struct HpetChannel {
    index: u8,
    irq: u8,
    internal: Arc<HpetInternal>,
}

#[async_trait::async_trait]
impl super::ArbitraryTimerTrait for HpetChannel {
    fn delay_ms_sync(&self, ms: u32) {
        let counts = (ms as u64) * 1_000_000_000_000;
        let ticks = counts / self.internal.period as u64;
        crate::VGA.print_str(&alloc::format!(
            "HPET NEEDS {} TICKS FOR {} ms\r\n",
            ticks,
            ms
        ));
        let mut this = self.internal.registers.sync_access();
        let newval = this.counter + ticks;
        this.channels[self.index as usize].comparator = newval;
        if this.counter >= newval {
            crate::VGA.print_str("HPET WONT TRIGGER IRQ\r\n");
        }
        this.channels[self.index as usize].config |= 4;
        loop {
            let curval = unsafe { core::ptr::read_volatile(&this.counter) };
            //crate::VGA.print_str(&alloc::format!("HPET {}/{}\r\n", curval, newval));
            if curval >= newval {
                break;
            }
        }
    }

    fn delay_us_sync(&self, us: u32) {
        todo!()
    }

    async fn delay_ms_async(&self, ms: u32) {
        crate::executor::dummy_future().await
    }
}

impl super::TimerInstanceInnerTrait for HpetChannel {
    fn supports_arbitrary_timing(&self) -> Option<&dyn super::ArbitraryTimerTrait> {
        Some(self)
    }

    fn get_guard_inner(&self) -> crate::IrqGuardedInner {
        IrqGuardedInner::new(
            crate::IrqNumbers::Only1(self.irq),
            false,
            true,
            |_| {},
            |_| {},
        )
    }

    fn hardware_interrupt(&self) -> u8 {
        self.irq
    }

    fn start_oneshot(&mut self) {
        let mut this = self.internal.registers.sync_access();
        let ticks = 100000;
        let newval = this.counter + ticks;
        this.channels[self.index as usize].comparator = newval;
        if this.counter >= newval {
            crate::VGA.print_str("HPET WONT TRIGGER\r\n");
        }
        this.channels[self.index as usize].config |= 4;
        loop {
            let curval = this.counter;
            crate::VGA.print_str(&alloc::format!("HPET {}/{}\r\n", curval, newval));
            if curval >= newval {
                break;
            }
        }
    }
}

/// An iterator over the hpet channels
pub struct HpetTimerIterator<'a> {
    /// current index
    cur: u8,
    /// Maximum index
    max: u8,
    /// internals
    internal: Arc<HpetInternal>,
    /// phantom
    phantom: PhantomData<&'a usize>,
}

impl<'a> Iterator for HpetTimerIterator<'a> {
    type Item = super::TimerInstanceInner;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.max {
            let t = HpetChannel {
                index: self.cur,
                irq: self.internal.irqs[self.cur as usize],
                internal: self.internal.clone(),
            };
            let r = Some(t.into());
            self.cur += 1;
            r
        } else {
            None
        }
    }
}

impl super::TimerTrait for Hpet {
    fn get_timer_inner(&mut self, i: u8) -> Result<super::TimerInstanceInner, super::TimerError> {
        if i < self.internal.num_channels {
            let h = HpetChannel {
                index: i,
                irq: self.internal.irqs[i as usize],
                internal: self.internal.clone(),
            };
            Ok(h.into())
        } else {
            Err(super::TimerError::InvalidTimerIndex)
        }
    }

    fn iter_mut(&mut self) -> super::TimerIterator<'_> {
        super::TimerIterator::Hpet(HpetTimerIterator {
            cur: 0,
            max: self.internal.num_channels,
            internal: self.internal.clone(),
            phantom: PhantomData,
        })
    }
}

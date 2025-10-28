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

struct HpetData {
    registers: &'static mut HpetRegisters,
    handlers: [Option<Arc<Box<super::TimerCallback2>>>; 32],
}

struct HpetInternal {
    data: IrqGuarded<HpetData>,
    /// irq numbers
    irqs: [u8; 32],
    /// The number of channels present for the hpet
    num_channels: u8,
    /// Period in femtoseconds
    period: u32,
}

impl HpetInternal {
    /// Get the number of ticks that corresponds to the specified number of milliseconds
    fn get_interval(&self, interval: u16) -> u64 {
        let counts = (interval as u64) * 1_000_000_000_000;
        let ticks = counts / self.period as u64;
        ticks
    }
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
        crate::VGA.print_str(&alloc::format!("HPET PERIOD IS {}\r\n", period));
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
        let d = HpetData {
            registers: r,
            handlers: [const { None }; 32],
        };
        let s = HpetInternal {
            data: IrqGuarded::new(d, &com),
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
            if !sys.register_irq_handler(*i, move || Self::handle_interrupt(&s2)) {
                crate::VGA.print_str("HPET INTERRUPT ALREADY TAKEN?\r\n");
            }
            sys.enable_irq(*i);
            crate::VGA.print_str(&alloc::format!("Enabled hpet irq {}\r\n", i));
        }
        let t = self.internal.data.sync_access().registers.counter;
        crate::VGA.print_str(&alloc::format!("HPET COUNT AT {}\r\n", t));
        let mut a = 0;
        let mut b;
        loop {
            let s = &mut self.internal.data.sync_access().registers;
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

    #[inline(never)]
    fn handle_interrupt(s: &Arc<HpetInternal>) {
        let mut this = s.data.interrupt_access();
        let intstat = this.registers.interrupt;
        for i in 0..s.num_channels {
            let val = 1 << i;
            if (intstat & val) != 0 {
                this.registers.interrupt = 1 << i;
                let handlers = &mut this.handlers;
                let h = handlers[i as usize].clone();
                drop(this);
                if let Some(h) = h {
                    h();
                }
                break;
            }
        }
    }
}

/// A single timer channel for the hpet timer
pub struct HpetChannel {
    index: u8,
    irq: u8,
    internal: Arc<HpetInternal>,
    interval: u64,
}

#[async_trait::async_trait]
impl super::ArbitraryTimerTrait for Arc<HpetChannel> {
    fn delay_ms_sync(&self, ms: u32) {
        let ticks = self.internal.get_interval(ms as u16);
        crate::VGA.print_str(&alloc::format!(
            "HPET NEEDS {} TICKS FOR {} ms\r\n",
            ticks,
            ms
        ));
        let this = &mut self.internal.data.sync_access().registers;
        let newval = this.counter + ticks;
        this.channels[self.index as usize].comparator = newval;
        if this.counter >= newval {
            crate::VGA.print_str("HPET WONT TRIGGER IRQ\r\n");
        }
        crate::VGA.print_str(&alloc::format!(
            "HPET CHANNEL CONFIG IS {:x}\r\n",
            this.channels[self.index as usize].config
        ));
        crate::VGA.print_str(&alloc::format!(
            "HPET GENERAL CONFIG IS {:x}\r\n",
            this.config
        ));
        crate::VGA.print_str(&alloc::format!(
            "HPET GENERAL COUNTER IS {}/{}\r\n",
            this.counter,
            newval,
        ));
        crate::VGA.print_str(&alloc::format!(
            "HPET GENERAL IRQSTAT IS {}\r\n",
            this.interrupt,
        ));
        loop {
            let curval = unsafe { core::ptr::read_volatile(&this.counter) };
            if curval >= newval {
                break;
            }
        }
        crate::VGA.print_str(&alloc::format!(
            "HPET GENERAL COUNTER IS {}\r\n",
            this.counter
        ));
    }

    fn delay_us_sync(&self, us: u32) {
        todo!()
    }

    async fn delay_ms_async(&self, ms: u32) {
        crate::executor::dummy_future().await
    }
}

impl super::TimerInstanceTrait for Arc<HpetChannel> {
    fn supports_arbitrary_timing(&self) -> Option<&dyn super::ArbitraryTimerTrait> {
        Some(self)
    }

    fn start_oneshot(&self) {
        let this = &mut self.internal.data.sync_access().registers;
        let ticks = self.interval;
        let newval = this.counter + ticks;
        this.channels[self.index as usize].comparator = newval;
        let config = this.channels[self.index as usize].config;
        unsafe {
            core::ptr::write_volatile(&mut this.channels[self.index as usize].config, config | 6)
        };
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
    type Item = super::TimerInstance;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.max {
            let t = HpetChannel {
                index: self.cur,
                irq: self.internal.irqs[self.cur as usize],
                internal: self.internal.clone(),
                interval: 100,
            };
            let t = Arc::new(t);
            let r = Some(t.into());
            self.cur += 1;
            r
        } else {
            None
        }
    }
}

impl super::TimerTrait for Hpet {
    fn get_timer(
        &mut self,
        i: u8,
        ms: u16,
        cb: Box<super::TimerCallback>,
    ) -> Result<super::TimerInstance, super::TimerError> {
        if i < self.internal.num_channels {
            let interval = self.internal.get_interval(ms);
            let h = HpetChannel {
                index: i,
                irq: self.internal.irqs[i as usize],
                internal: self.internal.clone(),
                interval,
            };
            let h = Arc::new(h);
            let h2: super::TimerInstance = h.into();
            let h3 = h2.clone();
            let c = &mut self.internal.data.sync_access().handlers;
            c[i as usize].replace(Arc::new(Box::new(move || cb(h3.clone()))));
            Ok(h2)
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

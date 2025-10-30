//! For the hpet (high performance timer) - normally found in x86 platforms

use alloc::{boxed::Box, vec::Vec};
use core::task::Waker;

use crate::{Arc, IrqGuarded, IrqGuardedInner, kernel::SystemTrait, modules::interrupt::InterruptControllerTrait};

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
    handlers: [super::TimerCallback2WithUsage; 32],
    wakers: [Option<Waker>; 32],
    channels_used: u32,
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
    fn get_interval_ms(&self, interval: u16) -> u64 {
        let counts = (interval as u64) * 1_000_000_000_000;
        let ticks = counts / self.period as u64;
        ticks
    }

    /// Get the number of ticks that corresponds to the specified number of microseconds
    fn get_interval_us(&self, interval: u16) -> u64 {
        let counts = (interval as u64) * 1_000_000_000;
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
                if let Some(irqnum) = crate::kernel::INTERRUPT_CONTROLLER.read().as_ref().unwrap().lookup_irq_with_channel(index) {
                    if crate::SYSTEM.read().register_irq_handler(irqnum, || {}) {
                        unsafe {
                            crate::SYSTEM.read().unregister_irq_handler(irqnum);
                        }
                        if ((rcap >> index) & 1) != 0 {
                            if !irqs.contains(&irqnum) {
                                irqs.push(irqnum);
                            }
                            crate::VGA.print_str(&alloc::format!(
                                "HPET CHANNEL {} USE IRQ {}\r\n",
                                i,
                                irqnum
                            ));
                            irq_values[i as usize] = irqnum;
                            break;
                        }
                    }
                }
            }
        }
        irqs.push(2);
        let irqnums = crate::IrqNumbers::Many(irqs.clone());
        let com = IrqGuardedInner::new(irqnums, true, true, |_| {}, |_| {});
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
            handlers: [const { super::TimerCallback2WithUsage::None }; 32],
            channels_used: 0,
            wakers: [const { None }; 32],
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
                if let Some(w) = this.wakers[i as usize].take() {
                    w.wake();
                }
                let handlers = &mut this.handlers;
                let h = handlers[i as usize].take();
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

/// The future used for delaying a certain period of time, asynchronously
struct HpetChannelFuture {
    index: u8,
    internal: Arc<HpetInternal>,
    newval: u64,
}

impl core::future::Future for HpetChannelFuture {
    type Output = ();
    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let timer_done = {
            let this = &mut self.internal.data.sync_access().registers;
            let curval = unsafe { core::ptr::read_volatile(&this.counter) };
            curval >= self.newval
        };
        if timer_done {
            return core::task::Poll::Ready(());
        } else {
            let mut this = self.internal.data.sync_access();
            if this.wakers[self.index as usize].is_none() {
                this.wakers[self.index as usize].replace(cx.waker().clone());
            }
            return core::task::Poll::Pending;
        }
    }
}

impl Drop for HpetChannel {
    fn drop(&mut self) {
        let mut this = self.internal.data.sync_access();
        let mut v = this.channels_used;
        v &= !(1 << self.index);
        crate::VGA.print_str(&alloc::format!("TIMER USAGE IS NOW {:X}\r\n", v));
        this.channels_used = v;
    }
}

impl Arc<HpetChannel> {
    /// Delay the specified number of ticks, synchronously
    fn delay_ticks(&self, ticks: u64) {
        let newval = {
            let this = &mut self.internal.data.sync_access().registers;
            let newval = this.counter + ticks;
            this.channels[self.index as usize].comparator = newval;
            newval
        };
        loop {
            let curval = {
                let a = self.internal.data.sync_access();
                unsafe { core::ptr::read_volatile(&a.registers.counter) }
            };
            if curval >= newval {
                break;
            }
            crate::scheduler::yield_task();
        }
    }
}

#[async_trait::async_trait]
impl super::ArbitraryTimerTrait for Arc<HpetChannel> {
    fn delay_ms_sync(&self, ms: u32) {
        let ticks = self.internal.get_interval_ms(ms as u16);
        crate::VGA.print_str(&alloc::format!(
            "HPET NEEDS {} TICKS FOR {} ms\r\n",
            ticks,
            ms
        ));
        self.delay_ticks(ticks);
    }

    fn delay_us_sync(&self, us: u32) {
        let ticks = self.internal.get_interval_us(us as u16);
        crate::VGA.print_str(&alloc::format!(
            "HPET NEEDS {} TICKS FOR {} us\r\n",
            ticks,
            us
        ));
        self.delay_ticks(ticks);
    }

    async fn delay_ms_async(&self, ms: u32) {
        let ticks = self.internal.get_interval_ms(ms as u16);
        let newval = {
            let this = &mut self.internal.data.sync_access().registers;
            let newval = this.counter + ticks;
            this.channels[self.index as usize].comparator = newval;
            this.channels[self.index as usize].comparator = newval;
            let config = this.channels[self.index as usize].config;
            unsafe {
                core::ptr::write_volatile(
                    &mut this.channels[self.index as usize].config,
                    config | 6,
                )
            };
            newval
        };
        let f = HpetChannelFuture {
            index: self.index,
            newval,
            internal: self.internal.clone(),
        };
        f.await;
    }
}

impl super::TimerInstanceTrait for Arc<HpetChannel> {
    fn supports_arbitrary_timing(&self) -> Option<&dyn super::ArbitraryTimerTrait> {
        Some(self)
    }

    fn manually_trigger(&self) {
        let this = &mut self.internal.data.sync_access().registers;
        let newval = this.counter;
        this.channels[self.index as usize].comparator = newval + 100;
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
pub struct HpetTimerIterator {
    /// current index
    cur: u8,
    /// Maximum index
    max: u8,
    /// internals
    internal: Arc<HpetInternal>,
}

impl Iterator for HpetTimerIterator {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur < self.max {
            let r = self.cur;
            self.cur += 1;
            Some(r)
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
        cb: super::TimerCallbackWithUsage,
    ) -> Result<super::TimerInstance, super::TimerError> {
        if i < self.internal.num_channels {
            if ((self.internal.data.sync_access().channels_used >> i) & 1) == 0 {
                let interval = self.internal.get_interval_ms(ms);
                let h = HpetChannel {
                    index: i,
                    irq: self.internal.irqs[i as usize],
                    internal: self.internal.clone(),
                    interval,
                };
                let h = Arc::new(h);
                let h2: super::TimerInstance = h.into();
                let h3 = h2.downgrade();
                {
                    let c = &mut self.internal.data.sync_access().handlers;
                    match cb {
                        super::TimerCallbackWithUsage::Single(a) => {
                            if let Some(a) = a {
                                c[i as usize] = super::TimerCallback2WithUsage::Single(Some(
                                    Arc::new(Box::new(move || a(h3.clone()))),
                                ));
                            }
                        }
                        super::TimerCallbackWithUsage::Multiple(a) => {
                            c[i as usize] = super::TimerCallback2WithUsage::Multiple(Arc::new(
                                Box::new(move || a(h3.clone())),
                            ));
                        }
                        super::TimerCallbackWithUsage::None => {}
                    }
                }
                self.internal.data.sync_access().channels_used |= 1 << i;
                crate::VGA.print_str(&alloc::format!(
                    "TIMER USAGE IS NOW2 {:X}\r\n",
                    self.internal.data.sync_access().channels_used
                ));
                Ok(h2)
            } else {
                Err(super::TimerError::TimerIsAlreadyUsed)
            }
        } else {
            Err(super::TimerError::InvalidTimerIndex)
        }
    }

    fn iter(&self) -> super::TimerIterator {
        super::TimerIterator::Hpet(HpetTimerIterator {
            cur: 0,
            max: self.internal.num_channels,
            internal: self.internal.clone(),
        })
    }
}

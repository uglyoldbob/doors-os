//! Serial port code for x86 serial ports

use core::future::Future;
use core::pin::Pin;
use core::task::Waker;

use alloc::boxed::Box;
use alloc::sync::Arc;

use crate::executor;
use crate::kernel::SystemTrait;
use crate::AsyncLockedArc;
use crate::IoPortArray;
use crate::IoReadWrite;
use crate::IO_PORT_MANAGER;

/// A serial port (COM) for x86
pub struct X86SerialPort {
    /// The io ports
    base: IoPortArray<'static>,
    /// The transmit queue
    tx_queue: Arc<conquer_once::spin::OnceCell<crossbeam::queue::ArrayQueue<u8>>>,
    /// The transmit wakers
    tx_wakers: Arc<conquer_once::spin::OnceCell<crossbeam::queue::ArrayQueue<Waker>>>,
    /// Are interrupts enabled?
    interrupts: bool,
    /// Is an interrupt driven transmission currently in progress?
    itx: bool,
    /// Irq number for interrupts
    irq: u8,
}

impl X86SerialPort {
    /// Attempt to build a new serial port, probing it as needed
    pub fn new(base: u16, irq: u8) -> Option<Self> {
        let ports = IO_PORT_MANAGER
            .as_ref()
            .unwrap()
            .get_ports(base, 8)
            .unwrap();
        //disable interrupts
        ports.port(1).port_write(0u8);
        //baud set to 115200
        ports.port(3).port_write(0x80u8);
        ports.port(0).port_write(1u8);
        ports.port(1).port_write(0u8);
        // Set data format
        ports.port(3).port_write(3u8);
        //enable fifo
        ports.port(2).port_write(0xc7u8);
        //enable loopback mode for probing
        ports.port(4).port_write(0x13u8);
        let testval = 0x55u8;
        ports.port(0).port_write(testval);

        let mut s = Self {
            base: ports,
            tx_queue: Arc::new(conquer_once::spin::OnceCell::uninit()),
            tx_wakers: Arc::new(conquer_once::spin::OnceCell::uninit()),
            interrupts: false,
            itx: false,
            irq,
        };
        let a = s.receive();
        if let Some(a) = a {
            if a == testval {
                s.setup();
                Some(s)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Check the transmit fifo to see if it is full
    fn can_send(&mut self) -> bool {
        let a: u8 = self.base.port(5).port_read();
        (a & 0x20) != 0
    }

    /// Check to see if there is a byte available
    fn can_receive(&mut self) -> bool {
        let a: u8 = self.base.port(5).port_read();
        (a & 0x01) != 0
    }

    /// Receive a byte
    fn receive(&mut self) -> Option<u8> {
        let mut attempts = 0;
        while !self.can_receive() {
            attempts += 1;
            if attempts == 60000 {
                return None;
            }
        }
        Some(self.base.port(0).port_read())
    }

    /// Setup the serial port
    fn setup(&mut self) {
        self.tx_queue
            .try_init_once(|| crossbeam::queue::ArrayQueue::new(32));
        self.tx_wakers
            .try_init_once(|| crossbeam::queue::ArrayQueue::new(32));
        // Enable interrupts for receiving data
        self.base.port(1).port_write(1u8);
        self.base.port(4).port_write(0x03u8);
    }

    /// The interrupt handler code
    fn handle_interrupt(s: &AsyncLockedArc<X86SerialPort>) {
        let mut s2 = s.sync_lock();
        if let Ok(aq) = s2.tx_queue.try_get() {
            if let Some(v) = aq.pop() {
                s2.base.port(0).port_write(v);
            }
            if aq.is_empty() {
                s2.disable_tx_interrupt();
            }
        }
        let _ = s2.tx_wakers.try_get().map(|a| {
            while let Some(w) = a.pop() {
                w.wake();
            }
        });
    }

    /// Enable the tx interrupt, used when sending data over the serial port
    /// * Safety: The irq should be disable when calling this function, otherwise the irq can happen before the object gets unlocked.
    unsafe fn enable_tx_interrupt(&mut self) {
        if !self.itx {
            let v: u8 = self.base.port(1).port_read();
            self.base.port(1).port_write(v | 2);
            self.itx = true;
        }
    }

    /// Stop the tx interrupt. Used when a transmission has completed.
    fn disable_tx_interrupt(&mut self) {
        if self.itx {
            let v: u8 = self.base.port(1).port_read();
            self.base.port(1).port_write(v & !2);
            self.itx = false;
        }
    }

    /// synchronously send a byte
    fn sync_send_byte(&mut self, c: u8) {
        while !self.can_send() {}
        self.base.port(0).port_write(c);
    }
}

impl AsyncLockedArc<X86SerialPort> {
    /// Asynchronously enable the tx interrupt.
    async fn enable_tx_interrupt(&self) {
        let (flag, irqnum) = {
            let s = self.lock().await;
            (s.itx, s.irq)
        };
        if !flag {
            if let Some(s) = crate::SYSTEM.lock().await.as_ref() {
                s.disable_irq(irqnum);
            }
            unsafe {
                self.lock().await.enable_tx_interrupt();
            }
            if let Some(s) = crate::SYSTEM.lock().await.as_ref() {
                s.enable_irq(irqnum);
            }
        }
    }
}

impl super::SerialTrait for AsyncLockedArc<X86SerialPort> {
    fn setup(&self, _rate: u32) -> Result<(), ()> {
        todo!();
    }

    fn enable_interrupts(&self) -> Result<(), ()> {
        let irqnum = {
            let mut s = self.sync_lock();
            s.base.port(4).port_write(0x03u8 | 8u8);
            s.interrupts = true;
            s.irq
        };
        let mut p = crate::SYSTEM.sync_lock();
        use crate::kernel::SystemTrait;
        let s2 = self.clone();
        p.as_mut().map(move |p| {
            p.register_irq_handler(irqnum, move || X86SerialPort::handle_interrupt(&s2));
            p.enable_irq(irqnum);
        });

        Ok(())
    }

    fn sync_transmit(&self, data: &[u8]) {
        let mut s = self.sync_lock();
        for c in data {
            s.sync_send_byte(*c);
        }
    }

    fn sync_transmit_str(&self, data: &str) {
        let mut s = self.sync_lock();
        for c in data.bytes() {
            s.sync_send_byte(c);
        }
    }

    fn sync_flush(&self) {}

    async fn transmit(&self, data: &[u8]) {
        AsyncWriter::new(self, data).await
    }

    async fn transmit_str(&self, data: &str) {
        AsyncWriter::new(self, data.as_bytes()).await
    }

    async fn flush(&self) {
        let s = self.lock().await;
        if let Some(q) = s.tx_queue.get() {
            while !q.is_empty() {
                executor::Task::yield_now().await;
            }
        }
    }
}

/// The async struct for serial port sending
#[pin_project::pin_project]
struct AsyncWriter<'a> {
    /// The array queue to write into
    s: &'a AsyncLockedArc<X86SerialPort>,
    /// The index into the data
    index: usize,
    /// The data reference
    data: &'a [u8],
    /// Waiting on interrupt enable
    interrupt_enable: bool,
    /// The interrupt enable future
    #[pin]
    ienable: futures::future::BoxFuture<'a, ()>,
}

impl<'a> AsyncWriter<'a> {
    /// Construct a new object for asynchronous serial port writing
    fn new(s: &'a AsyncLockedArc<X86SerialPort>, data: &'a [u8]) -> Self {
        Self {
            s,
            index: 0,
            data,
            interrupt_enable: false,
            ienable: Box::pin(s.enable_tx_interrupt()),
        }
    }
}

impl Future for AsyncWriter<'_> {
    type Output = ();
    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        if !self.interrupt_enable {
            let mut newindex = self.index;
            let this = self.s.sync_lock();
            let r2 = if let Some(q) = this.tx_queue.get() {
                loop {
                    if !q.is_full() {
                        if newindex < self.data.len() {
                            if q.push(self.data[newindex]).is_ok() {
                                newindex += 1;
                                if !self.interrupt_enable {
                                    self.interrupt_enable = true;
                                    self.ienable = Box::pin(self.s.enable_tx_interrupt());
                                }
                            } else {
                                let _ = this.tx_wakers.get().unwrap().push(cx.waker().clone());
                                break core::task::Poll::Pending;
                            }
                        } else if self.interrupt_enable {
                            if self.ienable.as_mut().poll(cx).is_ready() {
                                self.interrupt_enable = false;
                                break core::task::Poll::Ready(());
                            } else {
                                let _ = this.tx_wakers.get().unwrap().push(cx.waker().clone());
                                break core::task::Poll::Pending;
                            }
                        } else {
                            break core::task::Poll::Ready(());
                        }
                    } else if self.interrupt_enable {
                        let _ = this.tx_wakers.get().unwrap().push(cx.waker().clone());
                        if self.ienable.as_mut().poll(cx).is_ready() {
                            self.interrupt_enable = false;
                        }
                        break core::task::Poll::Pending;
                    }
                }
            } else {
                let _ = this.tx_wakers.get().unwrap().push(cx.waker().clone());
                core::task::Poll::Pending
            };
            drop(this);
            self.index = newindex;
            r2
        } else {
            if self.ienable.as_mut().poll(cx).is_ready() {
                self.interrupt_enable = false;
            }
            core::task::Poll::Pending
        }
    }
}

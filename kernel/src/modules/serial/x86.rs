//! Serial port code for x86 serial ports

use core::future::Future;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use core::task::Waker;

use crate::common;
use crate::Arc;

use crate::executor;
use crate::IoPortArray;
use crate::IoReadWrite;
use crate::IrqGuardedSimple;
use crate::IO_PORT_MANAGER;

/// The number of elements to store in the tx queue for each serial port
const TX_BUFFER_SIZE: usize = 1024;
/// The number of elements to store in the rx queue for each serial port
const RX_BUFFER_SIZE: usize = 1024;
/// the number of wakers to store for each queue
const NUM_WAKERS: usize = 32;

/// An x86 serial port
pub struct X86SerialPort(Arc<X86SerialPortInternal>);

/// A serial port (COM) for x86
pub struct X86SerialPortInternal {
    /// The io ports
    base: crate::IrqGuardedSimple<IoPortArray<'static>>,
    /// The transmit queue
    tx_queue: Arc<crate::IrqGuardedSimple<crossbeam::queue::ArrayQueue<u8>>>,
    /// The transmit wakers
    tx_wakers: Arc<crate::IrqGuardedSimple<crossbeam::queue::ArrayQueue<Waker>>>,
    /// The receive queue
    rx_queue: Arc<crate::IrqGuardedSimple<crossbeam::queue::ArrayQueue<u8>>>,
    /// The receive wakers
    rx_wakers: Arc<crate::IrqGuardedSimple<crossbeam::queue::ArrayQueue<Waker>>>,
    /// Is the tx interrupt currently enabled?
    tx_enabled: AtomicBool,
    /// Are interrupts enabled?
    interrupts: AtomicBool,
    /// Is an interrupt driven transmission currently in progress?
    itx: AtomicBool,
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

        let com = common::IrqGuardedInner::new(irq, true, true, |_| {}, |_| {});

        let i = Arc::new(X86SerialPortInternal {
            base: IrqGuardedSimple::new(ports, &com),
            tx_queue: Arc::new(IrqGuardedSimple::new(
                crossbeam::queue::ArrayQueue::new(TX_BUFFER_SIZE),
                &com,
            )),
            tx_wakers: Arc::new(IrqGuardedSimple::new(
                crossbeam::queue::ArrayQueue::new(NUM_WAKERS),
                &com,
            )),
            rx_queue: Arc::new(IrqGuardedSimple::new(
                crossbeam::queue::ArrayQueue::new(RX_BUFFER_SIZE),
                &com,
            )),
            rx_wakers: Arc::new(IrqGuardedSimple::new(
                crossbeam::queue::ArrayQueue::new(NUM_WAKERS),
                &com,
            )),
            tx_enabled: AtomicBool::new(false),
            interrupts: AtomicBool::new(false),
            itx: AtomicBool::new(false),
            irq,
        });
        let mut s = Self(i);
        let a = s.0.receive();
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
    fn can_send(&self) -> bool {
        let a: u8 = self.0.base.access().port(5).port_read();
        (a & 0x20) != 0
    }

    /// Setup the serial port
    fn setup(&mut self) {
        // Enable interrupts for receiving data
        self.0.base.access().port(1).port_write(1u8);
        self.0.base.access().port(4).port_write(0x03u8);
    }

    /// The interrupt handler code
    fn handle_interrupt(s: &Arc<X86SerialPortInternal>) {
        loop {
            x86_64::instructions::bochs_breakpoint();
            let stat: u8 = s.base.interrupt_access().port(2).port_read();
            if (stat & 1) == 0 {
                match (stat >> 1) & 7 {
                    0 => {
                        let _: u8 = s.base.interrupt_access().port(3).port_read();
                    }
                    1 => {
                        if let Some(v) = s.tx_queue.interrupt_access().pop() {
                            s.base.interrupt_access().port(0).port_write(v);
                        } else {
                            s.disable_tx_interrupt();
                        }
                        while let Some(w) = s.tx_wakers.interrupt_access().pop() {
                            w.wake();
                        }
                    }
                    2 | 6 => {
                        let recvd = s.base.interrupt_access().port(0).port_read();
                        let _ = s.rx_queue.interrupt_access().push(recvd);
                        while let Some(w) = s.tx_wakers.interrupt_access().pop() {
                            w.wake();
                        }
                    }
                    3 => {
                        let _: u8 = s.base.interrupt_access().port(5).port_read();
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }
    }

    /// Enable the rx interrupt, used when receiving data over the serial port
    fn enable_rx_interrupt(&self) {
        if self.0.interrupts.load(Ordering::Relaxed) {
            let p = self.0.base.access();
            let mut ie = p.port(1);
            let v: u8 = ie.port_read();
            ie.port_write(v | 1);
        }
    }

    /// synchronously send a byte
    fn sync_send_byte(&self, c: u8) {
        while !self.can_send() {}
        self.force_send_byte(c);
    }

    /// Send a byte because we already know the port is ready
    fn force_send_byte(&self, c: u8) {
        self.0.base.interrupt_access().port(0).port_write(c);
    }
}

impl Arc<X86SerialPortInternal> {
    /// Enable the tx interrupt, used when sending data over the serial port
    fn enable_tx_interrupt(&self) {
        if self.interrupts.load(Ordering::Relaxed) {
            if !self.tx_enabled.load(Ordering::Relaxed) {
                x86_64::instructions::bochs_breakpoint();
                let p = self.base.access();
                let mut ie = p.port(1);
                let v: u8 = ie.port_read();
                ie.port_write(v | 2);
                self.tx_enabled.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Stop the tx interrupt. Used when a transmission has completed. Only to be called from the interrupt handler!
    fn disable_tx_interrupt(&self) {
        if self.interrupts.load(Ordering::Relaxed) {
            let p = self.base.interrupt_access();
            let v: u8 = p.port(1).port_read();
            p.port(1).port_write(1 | (v & !2));
            self.tx_enabled.store(false, Ordering::Relaxed);
        }
    }

    /// Receive a byte
    fn receive(&self) -> Option<u8> {
        let mut attempts = 0;
        while !self.can_receive() {
            attempts += 1;
            if attempts == 60000 {
                return None;
            }
        }
        Some(self.force_receive())
    }

    /// Receive a byte without checking
    fn force_receive(&self) -> u8 {
        self.base.access().port(0).port_read()
    }

    /// Check to see if there is a byte available
    fn can_receive(&self) -> bool {
        let a: u8 = self.base.access().port(5).port_read();
        (a & 0x01) != 0
    }
}

/// A stream struct for receiving serial data
struct X86SerialStream {
    /// The data queue for the rx stream
    queue: Arc<IrqGuardedSimple<crossbeam::queue::ArrayQueue<u8>>>,
    /// The wakers for the rx stream
    wakers: Arc<IrqGuardedSimple<crossbeam::queue::ArrayQueue<Waker>>>,
}

impl futures::Stream for X86SerialStream {
    type Item = u8;
    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let a = self.queue.access().pop();
        if let Some(b) = a {
            core::task::Poll::Ready(Some(b))
        } else {
            self.wakers.access().push(cx.waker().clone()).unwrap();
            core::task::Poll::Pending
        }
    }
}

impl super::SerialTrait for X86SerialPort {
    fn setup(&self, _rate: u32) -> Result<(), ()> {
        todo!();
    }

    fn read_stream(&self) -> impl futures::Stream<Item = u8> {
        X86SerialStream {
            queue: self.0.rx_queue.clone(),
            wakers: self.0.rx_wakers.clone(),
        }
    }

    fn stop_async(&self) {
        use crate::kernel::SystemTrait;
        let irqnum = { self.0.irq };
        {
            self.0.interrupts.store(false, Ordering::Relaxed);
        };
        crate::SYSTEM.read().disable_irq(irqnum);
        self.0.base.access().port(1).port_write(0u8);
    }

    fn enable_async(&self, sys: crate::kernel::System) -> Result<(), ()> {
        use crate::kernel::SystemTrait;
        let irqnum = { self.0.irq };
        {
            let s2 = self.0.clone();
            sys.register_irq_handler(irqnum, move || X86SerialPort::handle_interrupt(&s2));
        }
        {
            self.0
                .base
                .interrupt_access()
                .port(4)
                .port_write(0x03u8 | 8u8);
            self.0.interrupts.store(true, Ordering::Relaxed);
        };
        self.enable_rx_interrupt();
        sys.enable_irq(irqnum);
        Ok(())
    }

    fn sync_transmit(&self, data: &[u8]) {
        if !self.0.interrupts.load(Ordering::Relaxed) {
            for c in data {
                self.sync_send_byte(*c);
            }
        } else {
            let txq = self.0.tx_queue.clone();
            self.0.itx.store(true, Ordering::Relaxed);
            let mut ienabled = false;
            for c in data.iter() {
                while txq.interrupt_access().is_full() {
                    for _ in 0..1000000 {
                        x86_64::instructions::nop();
                    }
                }
                txq.access().push(*c).unwrap();
                if true {
                    self.0.enable_tx_interrupt();
                    ienabled = true;
                }
                for _ in 0..1000000 {
                    x86_64::instructions::nop();
                }
            }
            if !ienabled {
                self.0.enable_tx_interrupt();
            }
            self.0.itx.store(false, Ordering::Relaxed);
        }
    }

    fn sync_transmit_str(&self, data: &str) {
        self.sync_transmit(data.as_bytes());
    }

    fn sync_flush(&self) {
        let i = self.0.interrupts.load(Ordering::Relaxed);
        if i {
            loop {
                let empty = self.0.tx_queue.access().is_empty();
                if empty {
                    break;
                }
                for _ in 0..1000000 {
                    x86_64::instructions::nop();
                }
            }
        }
    }

    async fn transmit(&self, data: &[u8]) {
        self.0.itx.store(true, Ordering::Relaxed);
        AsyncWriter::new(self.0.clone(), data, crate::SYSTEM.read().clone()).await
    }

    async fn transmit_str(&self, data: &str) {
        self.0.itx.store(true, Ordering::Relaxed);
        AsyncWriter::new(
            self.0.clone(),
            data.as_bytes(),
            crate::SYSTEM.read().clone(),
        )
        .await;
    }

    async fn flush(&self) {
        while !self.0.tx_queue.access().is_empty() {
            executor::Task::yield_now().await;
        }
    }
}

/// The async struct for serial port sending
struct AsyncWriter<'a> {
    /// The array queue to write into
    s: Arc<X86SerialPortInternal>,
    /// The index into the data
    index: usize,
    /// The data reference
    data: &'a [u8],
    /// The system
    sys: crate::kernel::System,
    /// Irq number
    irq: u8,
}

impl<'a> AsyncWriter<'a> {
    /// Construct a new object for asynchronous serial port writing
    fn new(s: Arc<X86SerialPortInternal>, data: &'a [u8], sys: crate::kernel::System) -> Self {
        let i = s.irq;
        Self {
            s,
            index: 0,
            data,
            sys: sys.clone(),
            irq: i,
        }
    }
}

impl Future for AsyncWriter<'_> {
    type Output = ();
    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let mut newindex = self.index;
        let mut interrupt_enable = false;
        let this = &self.s;
        if !this.interrupts.load(Ordering::Relaxed) {
            panic!("interrupts not enabled for future");
        }
        let tx_wakers = this.tx_wakers.clone();
        let queue = this.tx_queue.clone();
        let r2 = loop {
            let qfull = queue.access().is_full();
            if !qfull {
                if newindex < self.data.len() {
                    let a = queue.access().push(self.data[newindex]);
                    if a.is_ok() {
                        newindex += 1;
                        if !interrupt_enable {
                            interrupt_enable = true;
                        }
                    } else {
                        tx_wakers.access().push(cx.waker().clone());
                        break core::task::Poll::Pending;
                    }
                } else if interrupt_enable {
                    self.s.enable_tx_interrupt();
                    break core::task::Poll::Ready(());
                } else {
                    break core::task::Poll::Ready(());
                }
            } else {
                tx_wakers.access().push(cx.waker().clone());
                self.s.enable_tx_interrupt();
                break core::task::Poll::Pending;
            }
        };
        self.index = newindex;
        doors_macros::todo_item!("Remove this conditional code");
        if r2.is_ready() {
            self.s
                .itx
                .store(false, core::sync::atomic::Ordering::Relaxed);
        }
        r2
    }
}

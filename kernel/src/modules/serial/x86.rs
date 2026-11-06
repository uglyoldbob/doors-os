//! Serial port code for x86 serial ports

use core::future::Future;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
use core::task::Waker;

use crate::common;
use crate::new_irq_stream;
use crate::Arc;

use crate::executor;
use crate::IoPortArray;
use crate::IoReadWrite;
use crate::IrqGuardedSimple;
use crate::IrqNumbers;
use crate::IrqStreamReader;
use crate::IrqStreamWriter;
use crate::IO_PORT_MANAGER;

/// The number of elements to store in the tx queue for each serial port
const TX_BUFFER_SIZE: usize = 1024;
/// The number of elements to store in the rx queue for each serial port
const RX_BUFFER_SIZE: usize = 1024;
/// the number of wakers to store for each queue
const NUM_WAKERS: usize = 32;
/// How often (in iterations of the synchronous tx wait loops) the tx interrupt is re armed to
/// recover an edge the interrupt controller may have missed.
const TX_REARM_INTERVAL: u64 = 1 << 16;

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
    /// The rx stream
    rx_stream: (IrqStreamReader<u8>, IrqStreamWriter<u8>),
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

        let com = common::IrqGuardedInner::new(IrqNumbers::Only1(irq), true, false, |_| {}, |_| {});

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
            rx_stream: new_irq_stream(&com, RX_BUFFER_SIZE, NUM_WAKERS),
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
            let stat: u8 = s.base.interrupt_access().port(2).port_read();
            let _lsr: u8 = s.base.interrupt_access().port(5).port_read();
            let _: u8 = s.base.interrupt_access().port(6).port_read();
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
                        let _ = s.rx_stream.1.push_interrupt(recvd);
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
        if self.0.interrupts.load(Ordering::SeqCst) {
            let p = self.0.base.access();
            let mut ie = p.port(1);
            let v: u8 = ie.port_read();
            ie.port_write(v | 1);
        }
    }

    /// synchronously send a byte
    fn sync_send_byte(&self, c: u8) {
        while !self.can_send() {
            for _ in 0..100000 {
                crate::nop();
            }
        }
        self.force_send_byte(c);
    }

    /// Send a byte because we already know the port is ready
    fn force_send_byte(&self, c: u8) {
        self.0.base.interrupt_access().port(0).port_write(c);
    }
}

impl Arc<X86SerialPortInternal> {
    /// Enable the tx interrupt, used when sending data over the serial port.
    ///
    /// When `rearm` is set the ier bit is also cleared before it is set again. The uart only
    /// drives its interrupt output while a condition is pending, so an edge can be swallowed
    /// (the irq is masked at the controller while the line is asserted, or the legacy pic is
    /// replaced by the ioapic while a transmission is in flight) and an edge triggered
    /// controller will then never deliver it, leaving the tx queue waiting for an interrupt
    /// that never comes. Toggling the bit makes the uart drop and re assert the line, which
    /// gives the controller the transition it is waiting for.
    fn enable_tx_interrupt(&self, rearm: bool) {
        if self.interrupts.load(Ordering::SeqCst)
            && (rearm || !self.tx_enabled.load(Ordering::SeqCst))
        {
            // The guard keeps interrupts off so that the handler cannot run between the two
            // writes below.
            let p = self.base.access();
            let mut ie = p.port(1);
            let v: u8 = ie.port_read();
            if rearm {
                ie.port_write(v & !2);
            }
            ie.port_write(v | 2);
            self.tx_enabled.store(true, Ordering::SeqCst);
        }
    }

    /// Stop the tx interrupt. Used when a transmission has completed. Only to be called from the interrupt handler!
    fn disable_tx_interrupt(&self) {
        if self.interrupts.load(Ordering::SeqCst) {
            let p = self.base.interrupt_access();
            let v: u8 = p.port(1).port_read();
            p.port(1).port_write(1 | (v & !2));
            self.tx_enabled.store(false, Ordering::SeqCst);
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

impl super::SerialTrait for X86SerialPort {
    fn setup(&self, _rate: u32) -> Result<(), ()> {
        todo!();
    }

    fn sync_read_byte(&self) -> u8 {
        if !self.0.interrupts.load(Ordering::SeqCst) {
            while !self.0.can_receive() {}
            self.0.receive().unwrap()
        } else {
            loop {
                if let Some(a) = self.0.rx_stream.0.get_next() {
                    return a;
                }
            }
        }
    }

    fn try_read(&self) -> Option<u8> {
        if !self.0.interrupts.load(Ordering::SeqCst) {
            if self.0.can_receive() {
                self.0.receive()
            } else {
                None
            }
        } else {
            self.0.rx_stream.0.get_next()
        }
    }

    fn read_stream(&self) -> impl futures::Stream<Item = u8> {
        &self.0.rx_stream.0
    }

    fn stop_async(&self) {
        use crate::kernel::SystemTrait;
        let irqnum = { self.0.irq };
        {
            self.0.interrupts.store(false, Ordering::SeqCst);
        };
        crate::SYSTEM.read().disable_irq(irqnum);
        self.0.base.interrupt_access().port(1).port_write(0u8);
        // The uart is no longer armed to produce tx interrupts, so a later enable_async must
        // not believe the tx interrupt is still turned on.
        self.0.tx_enabled.store(false, Ordering::SeqCst);
    }

    fn enable_async(&self, sys: crate::kernel::System) -> Result<(), ()> {
        use crate::kernel::SystemTrait;
        let irqnum = { self.0.irq };
        // Start from a known state with the uart interrupt output turned off. An edge created
        // while the interrupt controller still has this irq masked is lost for good (the ioapic
        // entry for a com port is edge triggered), so nothing may assert the irq line until the
        // irq is really wired up.
        {
            let p = self.0.base.access();
            p.port(1).port_write(0u8);
            p.port(4).port_write(0x03u8);
        }
        self.0.tx_enabled.store(false, Ordering::SeqCst);
        self.0.interrupts.store(false, Ordering::SeqCst);
        // Throw away anything already waiting in the receiver and read the status registers so
        // that no interrupt is left latched in the uart. The bound keeps a stuck receive line
        // from wedging us here.
        for _ in 0..RX_BUFFER_SIZE {
            if !self.0.can_receive() {
                break;
            }
            self.0.force_receive();
        }
        {
            let p = self.0.base.access();
            let _stat: u8 = p.port(2).port_read();
            let _lsr: u8 = p.port(5).port_read();
            let _msr: u8 = p.port(6).port_read();
        }
        // Wire the irq before anything is allowed to request interrupt driven tx or rx.
        let s2 = self.0.clone();
        if !sys.register_irq_handler(irqnum, move || X86SerialPort::handle_interrupt(&s2)) {
            // Another device already owns this irq (com1/com3 and com2/com4 share them), so
            // nothing would service this port's interrupts. Stay in polling mode rather than
            // arming an irq that will never drain the tx queue.
            return Err(());
        }
        sys.enable_irq(irqnum);
        // Only now is it safe to announce that interrupt driven operation is available.
        self.0.interrupts.store(true, Ordering::SeqCst);
        self.enable_rx_interrupt();
        // Kick off anything that was left in the queue while the port was in polling mode.
        if !self.0.tx_queue.access().is_empty() {
            self.0.enable_tx_interrupt(true);
        }
        // Turn the uart interrupt output on last so that any condition that is already pending
        // produces a fresh edge that the now unmasked controller will see.
        self.0.base.access().port(4).port_write(0x03u8 | 8u8);
        Ok(())
    }

    fn sync_transmit(&self, data: &[u8]) {
        if !self.0.interrupts.load(Ordering::SeqCst) {
            for c in data {
                self.sync_send_byte(*c);
            }
        } else {
            let txq = self.0.tx_queue.clone();
            self.0.itx.store(true, Ordering::SeqCst);
            let mut ienabled = false;
            for c in data.iter() {
                let mut spins: u64 = 0;
                while txq.interrupt_access().is_full() {
                    // Something has to keep the drain alive: if the interrupt controller never
                    // delivered the edge this queue would stay full forever.
                    self.0.enable_tx_interrupt(spins % TX_REARM_INTERVAL == 0);
                    spins += 1;
                }
                txq.access().push(*c).unwrap();
                if true {
                    self.0.enable_tx_interrupt(false);
                    ienabled = true;
                }
            }
            if !ienabled {
                self.0.enable_tx_interrupt(false);
            }
            self.0.itx.store(false, Ordering::SeqCst);
        }
    }

    fn sync_transmit_str(&self, data: &str) {
        self.sync_transmit(data.as_bytes());
    }

    fn sync_flush(&self) {
        let i = self.0.interrupts.load(Ordering::SeqCst);
        if i {
            let mut spins: u64 = 0;
            loop {
                let empty = self.0.tx_queue.interrupt_access().is_empty();
                if empty {
                    break;
                }
                // Re arm periodically so a tx edge that never reached the controller cannot
                // leave this flush waiting forever.
                self.0.enable_tx_interrupt(spins % TX_REARM_INTERVAL == 0);
                spins += 1;
            }
        }
    }

    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn transmit(&self, data: &[u8]) {
        self.0.itx.store(true, Ordering::SeqCst);
        AsyncWriter::new(self.0.clone(), data).await
    }

    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn transmit_str(&self, data: &str) {
        self.0.itx.store(true, Ordering::SeqCst);
        AsyncWriter::new(self.0.clone(), data.as_bytes()).await;
    }

    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn flush(&self) {
        let mut spins: u64 = 0;
        while !self.0.tx_queue.access().is_empty() {
            // Nothing else guarantees the queue keeps draining here, so make sure the uart is
            // still going to interrupt for whatever is left in it.
            self.0.enable_tx_interrupt(spins % TX_REARM_INTERVAL == 0);
            spins += 1;
            executor::AsyncTask::yield_now().await;
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
}

impl<'a> AsyncWriter<'a> {
    /// Construct a new object for asynchronous serial port writing
    fn new(s: Arc<X86SerialPortInternal>, data: &'a [u8]) -> Self {
        Self { s, index: 0, data }
    }
}

impl Future for AsyncWriter<'_> {
    type Output = ();
    #[track_caller]
    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let mut newindex = self.index;
        let mut interrupt_enable = false;
        let this = &self.s;
        if !this.interrupts.load(Ordering::SeqCst) {
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
                        let _ = tx_wakers.access().push(cx.waker().clone());
                        self.s.enable_tx_interrupt(true);
                        break core::task::Poll::Pending;
                    }
                } else if interrupt_enable {
                    self.s.enable_tx_interrupt(true);
                    break core::task::Poll::Ready(());
                } else {
                    break core::task::Poll::Ready(());
                }
            } else {
                let _ = tx_wakers.access().push(cx.waker().clone());
                self.s.enable_tx_interrupt(true);
                break core::task::Poll::Pending;
            }
        };
        self.index = newindex;
        doors_macros::todo_item!("Remove this conditional code");
        if r2.is_ready() {
            self.s
                .itx
                .store(false, core::sync::atomic::Ordering::SeqCst);
        }
        r2
    }
}

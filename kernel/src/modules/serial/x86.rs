//! Serial port code for x86 serial ports

use core::future::Future;
use core::task::Waker;

use spin::Mutex;
use spin::RwLock;

use crate::Arc;

use crate::executor;
use crate::kernel::SystemTrait;
use crate::IoPortArray;
use crate::IoPortRef;
use crate::IoReadWrite;
use crate::IO_PORT_MANAGER;

/// An x86 serial port
pub struct X86SerialPort(Arc<X86SerialPortInternal>);

/// A serial port (COM) for x86
pub struct X86SerialPortInternal {
    /// The io ports
    base: IoPortArray<'static>,
    /// The transmit queue
    tx_queue: Arc<conquer_once::spin::OnceCell<crossbeam::queue::ArrayQueue<u8>>>,
    /// The transmit wakers
    tx_wakers: Arc<conquer_once::spin::OnceCell<crossbeam::queue::ArrayQueue<Waker>>>,
    /// The receive queue
    rx_queue: Arc<conquer_once::spin::OnceCell<crossbeam::queue::ArrayQueue<u8>>>,
    /// The receive wakers
    rx_wakers: Arc<conquer_once::spin::OnceCell<crossbeam::queue::ArrayQueue<Waker>>>,
    /// Are interrupts enabled?
    interrupts: RwLock<bool>,
    /// Is an interrupt driven transmission currently in progress?
    itx: RwLock<bool>,
    /// Irq number for interrupts
    irq: u8,
    /// Interrupt enable port
    ienable: Mutex<IoPortRef<u8>>,
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

        let ienable = Mutex::new(ports.port(1));

        let i = Arc::new(X86SerialPortInternal {
            base: ports,
            tx_queue: Arc::new(conquer_once::spin::OnceCell::uninit()),
            tx_wakers: Arc::new(conquer_once::spin::OnceCell::uninit()),
            rx_queue: Arc::new(conquer_once::spin::OnceCell::uninit()),
            rx_wakers: Arc::new(conquer_once::spin::OnceCell::uninit()),
            interrupts: RwLock::new(false),
            itx: RwLock::new(false),
            irq,
            ienable,
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
        let a: u8 = self.0.base.port(5).port_read();
        (a & 0x20) != 0
    }

    /// Setup the serial port
    fn setup(&mut self) {
        self.0
            .tx_queue
            .try_init_once(|| crossbeam::queue::ArrayQueue::new(32))
            .unwrap();
        self.0
            .tx_wakers
            .try_init_once(|| crossbeam::queue::ArrayQueue::new(32))
            .unwrap();
        self.0
            .rx_queue
            .try_init_once(|| crossbeam::queue::ArrayQueue::new(32))
            .unwrap();
        self.0
            .rx_wakers
            .try_init_once(|| crossbeam::queue::ArrayQueue::new(32))
            .unwrap();
        // Enable interrupts for receiving data
        self.0.base.port(1).port_write(1u8);
        self.0.base.port(4).port_write(0x03u8);
    }

    /// The interrupt handler code
    fn handle_interrupt(s: &Arc<X86SerialPortInternal>) {
        let stat: u8 = s.base.port(2).port_read();
        match (stat >> 1) & 3 {
            1 => {
                if let Ok(aq) = s.tx_queue.try_get() {
                    if let Some(v) = aq.pop() {
                        s.base.port(0).port_write(v);
                    } else {
                        s.disable_tx_interrupt();
                    }
                }
                if let Ok(a) = s.tx_wakers.try_get() {
                    while let Some(w) = a.pop() {
                        w.wake();
                    }
                }
            }
            2 => {
                if let Ok(rq) = s.rx_queue.try_get() {
                    if let Some(a) = s.receive() {
                        let _ = rq.push(a);
                        if let Ok(a) = s.rx_wakers.try_get() {
                            while let Some(w) = a.pop() {
                                w.wake();
                            }
                        }
                    }
                }
            }
            _ => {
                x86_64::instructions::bochs_breakpoint();
            }
        }
    }

    /// Enable the rx interrupt, used when receiving data over the serial port
    /// * Safety: The irq should be disable when calling this function, otherwise the irq can happen before the object gets unlocked.
    unsafe fn enable_rx_interrupt(&self) {
        if *self.0.interrupts.read() {
            let _: u8 = self.0.base.port(2).port_read();
            let mut ie = self.0.ienable.lock();
            let v: u8 = ie.port_read();
            ie.port_write(v | 1);
        }
    }

    /// Return the status of the interrupt enable register
    fn read_tx_int_status(&self) -> u8 {
        self.0.base.port(1).port_read()
    }

    /// Return the status of the line status register
    fn read_tx_line_status(&self) -> u8 {
        self.0.base.port(5).port_read()
    }

    /// synchronously send a byte
    fn sync_send_byte(&self, c: u8) {
        while !self.can_send() {}
        self.force_send_byte(c);
    }

    /// Send a byte because we already know the port is ready
    fn force_send_byte(&self, c: u8) {
        self.0.base.port(0).port_write(c);
    }

    /// Asynchronously enable the tx interrupt.
    async fn enable_tx_interrupt(&self, sys: crate::kernel::System) {
        let (ie, irqnum) = { (*self.0.interrupts.read(), self.0.irq) };
        if ie {
            sys.disable_irq(irqnum);
            {
                unsafe {
                    self.0.internal_enable_tx_interrupt();
                }
            }
            sys.enable_irq(irqnum);
        }
    }
}

impl Arc<X86SerialPortInternal> {
    /// Enable the tx interrupt, used when sending data over the serial port
    /// * Safety: The irq should be disable when calling this function, otherwise the irq can happen before the object gets unlocked.
    unsafe fn internal_enable_tx_interrupt(&self) {
        if *self.interrupts.read() {
            let _: u8 = self.base.port(2).port_read();
            let mut ie = self.ienable.lock();
            let v: u8 = ie.port_read();
            ie.port_write(v | 2);
        }
    }

    /// Synchronous version of enable_tx_interrupt
    fn sync_enable_tx_interrupt(&self, sys: &crate::kernel::System) {
        let (ie, irqnum) = { (*self.interrupts.read(), self.irq) };
        if ie {
            sys.disable_irq(irqnum);
            {
                unsafe {
                    self.internal_enable_tx_interrupt();
                }
            }
            sys.enable_irq(irqnum);
        }
    }

    /// Stop the tx interrupt. Used when a transmission has completed.
    fn disable_tx_interrupt(&self) {
        if *self.interrupts.read() {
            let v: u8 = self.base.port(1).port_read();
            self.base.port(1).port_write(v & !2);
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
        Some(self.base.port(0).port_read())
    }

    /// Check to see if there is a byte available
    fn can_receive(&self) -> bool {
        let a: u8 = self.base.port(5).port_read();
        (a & 0x01) != 0
    }
}

/// A stream struct for receiving serial data
struct X86SerialStream {
    /// The data queue for the rx stream
    queue: Arc<conquer_once::spin::OnceCell<crossbeam::queue::ArrayQueue<u8>>>,
    /// The wakers for the rx stream
    wakers: Arc<conquer_once::spin::OnceCell<crossbeam::queue::ArrayQueue<Waker>>>,
}

impl futures::Stream for X86SerialStream {
    type Item = u8;
    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        if let Ok(q) = self.queue.try_get() {
            if let Some(b) = q.pop() {
                core::task::Poll::Ready(Some(b))
            } else {
                let ws = self.wakers.get().unwrap();
                ws.push(cx.waker().clone()).unwrap();
                core::task::Poll::Pending
            }
        } else {
            panic!();
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

    fn enable_async(&self, sys: crate::kernel::System) -> Result<(), ()> {
        use crate::kernel::SystemTrait;
        let irqnum = { self.0.irq };
        {
            let s2 = self.0.clone();
            sys.register_irq_handler(irqnum, move || X86SerialPort::handle_interrupt(&s2));
        }
        {
            self.0.base.port(4).port_write(0x03u8 | 8u8);
            *self.0.interrupts.write() = true;
        };
        unsafe { self.enable_rx_interrupt() };
        sys.enable_irq(irqnum);
        Ok(())
    }

    fn sync_transmit(&self, data: &[u8]) {
        if !*self.0.interrupts.read() {
            for c in data {
                self.sync_send_byte(*c);
            }
        } else {
            use alloc::borrow::ToOwned;
            let txq = self.0.tx_queue.clone();
            *self.0.itx.write() = true;
            let mut ienabled = false;
            let sys = crate::SYSTEM.sync_lock().to_owned().unwrap();
            for (i, c) in data.iter().enumerate() {
                if let Ok(tx) = txq.try_get() {
                    while tx.push(*c).is_err() {}
                    if i >= 8 {
                        self.0.sync_enable_tx_interrupt(&sys);
                        ienabled = true;
                    }
                }
            }
            if !ienabled {
                self.0.sync_enable_tx_interrupt(&sys);
            }
            *self.0.itx.write() = false;
        }
    }

    fn sync_transmit_str(&self, data: &str) {
        self.sync_transmit(data.as_bytes());
    }

    fn sync_flush(&self) {
        let (i, txq) = { (*self.0.interrupts.read(), self.0.tx_queue.clone()) };
        if i {
            if let Ok(tx) = txq.try_get() {
                while !tx.is_empty() {}
            }
        }
    }

    async fn transmit(&self, data: &[u8]) {
        use alloc::borrow::ToOwned;
        *self.0.itx.write() = true;
        AsyncWriter::new(
            self.0.clone(),
            data,
            crate::SYSTEM.sync_lock().to_owned().unwrap(),
        )
        .await
    }

    async fn transmit_str(&self, data: &str) {
        use alloc::borrow::ToOwned;
        *self.0.itx.write() = true;
        AsyncWriter::new(
            self.0.clone(),
            data.as_bytes(),
            crate::SYSTEM.sync_lock().to_owned().unwrap(),
        )
        .await;
    }

    async fn flush(&self) {
        if let Some(q) = self.0.tx_queue.get() {
            while !q.is_empty() {
                executor::Task::yield_now().await;
            }
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
}

impl<'a> AsyncWriter<'a> {
    /// Construct a new object for asynchronous serial port writing
    fn new(s: Arc<X86SerialPortInternal>, data: &'a [u8], sys: crate::kernel::System) -> Self {
        Self {
            s,
            index: 0,
            data,
            sys: sys.clone(),
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
        if !*this.interrupts.read() {
            panic!("interrupts not enabled for future");
        }
        let tx_wakers = this.tx_wakers.clone();
        let queue = this.tx_queue.clone();
        let r2 = if let Some(q) = queue.get() {
            loop {
                if !q.is_full() {
                    if newindex < self.data.len() {
                        if q.push(self.data[newindex]).is_ok() {
                            newindex += 1;
                            if !interrupt_enable {
                                interrupt_enable = true;
                            }
                        } else {
                            let _ = tx_wakers.get().unwrap().push(cx.waker().clone());
                            break core::task::Poll::Pending;
                        }
                    } else if interrupt_enable {
                        self.s.sync_enable_tx_interrupt(&self.sys);
                        break core::task::Poll::Ready(());
                    } else {
                        break core::task::Poll::Ready(());
                    }
                } else {
                    let _ = tx_wakers.get().unwrap().push(cx.waker().clone());
                    self.s.sync_enable_tx_interrupt(&self.sys);
                    break core::task::Poll::Pending;
                }
            }
        } else {
            let _ = tx_wakers.get().unwrap().push(cx.waker().clone());
            core::task::Poll::Pending
        };
        self.index = newindex;
        doors_macros::todo_item!("Remove this conditional code");
        if r2.is_ready() {
            *self.s.itx.write() = false;
            if queue.get().unwrap().is_empty() {
                self.s.disable_tx_interrupt();
            }
        }
        r2
    }
}

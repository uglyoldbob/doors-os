//! Serial port code for x86 serial ports

use core::future::Future;
use core::task::Waker;

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
    fn can_send(&self) -> bool {
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
        crate::VGA2.print_str("\tSerial port interrupt handler\r\n");
        let mut s2 = s.sync_lock();
        let stat: u8 = s2.base.port(2).port_read();
        if let Ok(aq) = s2.tx_queue.try_get() {
            if let Some(v) = aq.pop() {
                while !s2.can_send() {}
                crate::VGA2.print_fixed_str(doors_macros2::fixed_string_format!(
                    "\tSerial port sending a byte {} now\r\n",
                    v as char
                ));
                s2.base.port(0).port_write(v);
            } else {
                crate::VGA2
                    .print_str("\tSerial port interrupt handler did not get a byte to send\r\n");
                if aq.is_empty() {
                    crate::VGA2.print_str("\tSerial port queue is empty\r\n");
                }
                if aq.is_full() {
                    crate::VGA2.print_str("\ttSerial port queue is full?\r\n");
                }
                s2.disable_tx_interrupt();
            }
        }
        if let Ok(a) = s2.tx_wakers.try_get() {
            crate::VGA2.print_str("\tHandler waking all wakers\r\n");
            let mut index = 0;
            while let Some(w) = a.pop() {
                crate::VGA2.print_fixed_str(doors_macros2::fixed_string_format!(
                    "\tWaking waker {}\r\n",
                    index
                ));
                w.wake();
                index += 1;
            }
        }
    }

    /// Enable the tx interrupt, used when sending data over the serial port
    /// * Safety: The irq should be disable when calling this function, otherwise the irq can happen before the object gets unlocked.
    unsafe fn enable_tx_interrupt(&mut self) {
        if self.interrupts {
            crate::VGA2.print_str("\tSerial port tx interrupt enabled\r\n");
            let v: u8 = self.base.port(1).port_read();
            self.base.port(1).port_write(v | 2);
        }
    }

    /// Return the status of the interrupt enable register
    fn read_tx_int_status(&self) -> u8 {
        self.base.port(1).port_read()
    }

    /// Return the status of the line status register
    fn read_tx_line_status(&self) -> u8 {
        self.base.port(5).port_read()
    }

    /// Stop the tx interrupt. Used when a transmission has completed.
    fn disable_tx_interrupt(&mut self) {
        if self.interrupts {
            crate::VGA2.print_str("\tSerial port tx interrupt disabled\r\n");
            let v: u8 = self.base.port(1).port_read();
            self.base.port(1).port_write(v & !2);
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
    async fn enable_tx_interrupt(&self, sys: crate::kernel::System) {
        let (ie, irqnum) = {
            let s = self.lock().await;
            (s.interrupts, s.irq)
        };
        if ie {
            sys.disable_irq(irqnum);
            {
                unsafe {
                    self.lock().await.enable_tx_interrupt();
                }
            }
            sys.enable_irq(irqnum);
        }
    }

    /// Synchronous version of enable_tx_interrupt
    fn sync_enable_tx_interrupt(&self, sys: &crate::kernel::System) {
        let (ie, irqnum) = {
            let s = self.sync_lock();
            (s.interrupts, s.irq)
        };
        if ie {
            sys.disable_irq(irqnum);
            {
                unsafe {
                    self.sync_lock().enable_tx_interrupt();
                }
            }
            sys.enable_irq(irqnum);
        }
    }
}

impl super::SerialTrait for AsyncLockedArc<X86SerialPort> {
    fn setup(&self, _rate: u32) -> Result<(), ()> {
        todo!();
    }

    fn enable_async(&self, sys: crate::kernel::System) -> Result<(), ()> {
        use crate::kernel::SystemTrait;
        let irqnum = {
            let s = self.sync_lock();
            s.irq
        };
        {
            let s2 = self.clone();
            sys.register_irq_handler(irqnum, move || X86SerialPort::handle_interrupt(&s2));
        }
        {
            let mut s = self.sync_lock();
            s.base.port(4).port_write(0x03u8 | 8u8);
            s.interrupts = true;
        };
        sys.enable_irq(irqnum);
        Ok(())
    }

    fn sync_transmit(&self, data: &[u8]) {
        if self.sync_lock().irq == 4 {
            crate::VGA2.print_str("SYNC writing data: ");
            for b in data {
                crate::VGA2.print_fixed_str(doors_macros2::fixed_string_format!("{}", *b as char));
            }
        }
        let mut s = self.sync_lock();
        if !s.interrupts {
            for c in data {
                s.sync_send_byte(*c);
            }
        } else {
            use alloc::borrow::ToOwned;
            let txq = s.tx_queue.clone();
            s.itx = true;
            drop(s);
            let mut ienabled = false;
            let sys = crate::SYSTEM.sync_lock().to_owned().unwrap();
            for (i, c) in data.iter().enumerate() {
                if let Ok(tx) = txq.try_get() {
                    while tx.push(*c).is_err() {}
                    crate::VGA2.print_str(&alloc::format!(
                        "Submitted a byte {} to the serial queue\r\n",
                        c
                    ));
                    if i >= 8 {
                        self.sync_enable_tx_interrupt(&sys);
                        ienabled = true;
                    }
                }
            }
            if !ienabled {
                self.sync_enable_tx_interrupt(&sys);
            }
            self.sync_lock().itx = false;
        }
    }

    fn sync_transmit_str(&self, data: &str) {
        self.sync_transmit(data.as_bytes());
    }

    fn sync_flush(&self) {
        let (i, txq) = {
            let s = self.sync_lock();
            (s.interrupts, s.tx_queue.clone())
        };
        if i {
            if let Ok(tx) = txq.try_get() {
                while !tx.is_empty() {}
            }
        }
    }

    async fn transmit(&self, data: &[u8]) {
        use alloc::borrow::ToOwned;
        crate::VGA2.print_str("Async writing data: ");
        for b in data {
            crate::VGA2.print_fixed_str(doors_macros2::fixed_string_format!("{}", *b as char));
        }
        self.sync_lock().itx = true;
        AsyncWriter::new(self, data, crate::SYSTEM.sync_lock().to_owned().unwrap()).await
    }

    async fn transmit_str(&self, data: &str) {
        use alloc::borrow::ToOwned;
        crate::VGA2.print_str("Async writing string: ");
        crate::VGA2.print_str(data);
        self.sync_lock().itx = true;
        AsyncWriter::new(
            self,
            data.as_bytes(),
            crate::SYSTEM.sync_lock().to_owned().unwrap(),
        )
        .await;
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
struct AsyncWriter<'a> {
    /// The array queue to write into
    s: &'a AsyncLockedArc<X86SerialPort>,
    /// The index into the data
    index: usize,
    /// The data reference
    data: &'a [u8],
    /// The system
    sys: crate::kernel::System,
}

impl<'a> AsyncWriter<'a> {
    /// Construct a new object for asynchronous serial port writing
    fn new(
        s: &'a AsyncLockedArc<X86SerialPort>,
        data: &'a [u8],
        sys: crate::kernel::System,
    ) -> Self {
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
        crate::VGA2.print_str("Polling the x86 serial port\r\n");
        let mut newindex = self.index;
        let mut interrupt_enable = false;
        let this = self.s.sync_lock();
        if !this.interrupts {
            panic!("interrupts not enabled for future");
        }
        let tx_wakers = this.tx_wakers.clone();
        let queue = this.tx_queue.clone();
        drop(this);
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
                            crate::VGA2.print_str("Pending 3\r\n");
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
                    crate::VGA2.print_str("Pending 2\r\n");
                    break core::task::Poll::Pending;
                }
            }
        } else {
            let _ = tx_wakers.get().unwrap().push(cx.waker().clone());
            crate::VGA2.print_str("Pending 1\r\n");
            core::task::Poll::Pending
        };
        self.index = newindex;
        crate::VGA2.print_fixed_str(doors_macros2::fixed_string_format!(
            "TX QUEUE IS EMPTY? -> {}\r\n",
            queue.get().unwrap().is_empty()
        ));
        crate::VGA2.print_fixed_str(doors_macros2::fixed_string_format!(
            "TX QUEUE VAL1 -> 0x{:x}\r\n",
            self.s.sync_lock().read_tx_int_status()
        ));
        crate::VGA2.print_fixed_str(doors_macros2::fixed_string_format!(
            "TX QUEUE VAL2 -> 0x{:x}\r\n",
            self.s.sync_lock().read_tx_line_status()
        ));
        if r2.is_pending() {
            crate::VGA2.print_str("Polling the x86 serial port is pending\r\n");
        }
        if r2.is_ready() {
            self.s.sync_lock().itx = false;
            if queue.get().unwrap().is_empty() {
                self.s.sync_lock().disable_tx_interrupt();
            }
            crate::VGA2.print_str("Polling the x86 serial port is ready\r\n");
        }
        r2
    }
}

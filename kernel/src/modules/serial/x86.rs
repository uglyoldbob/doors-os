//! Serial port code for x86 serial ports

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
}

impl AsyncLockedArc<X86SerialPort> {
    /// Asynchronously enable the tx interrupt.
    async fn enable_tx_interrupt(&self) {
        crate::SYSTEM
            .lock()
            .await
            .as_ref()
            .map(|s| s.disable_irq(4));
        unsafe {
            self.lock().await.enable_tx_interrupt();
        }
        crate::SYSTEM.lock().await.as_ref().map(|s| s.enable_irq(4));
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
            while !s.can_send() {}
            s.base.port(0).port_write(*c);
        }
    }

    fn sync_transmit_str(&self, data: &str) {
        let mut s = self.sync_lock();
        for c in data.bytes() {
            while !s.can_send() {}
            s.base.port(0).port_write(c);
        }
    }

    fn sync_flush(&self) {}

    async fn transmit(&self, data: &[u8]) {
        let txq = {
            let s = self.lock().await;
            s.tx_queue.clone()
        };
        if let Some(q) = txq.get() {
            for c in data {
                while q.push(*c).is_err() {
                    executor::Task::yield_now().await;
                }
                self.enable_tx_interrupt().await;
            }
        }
    }

    async fn transmit_str(&self, data: &str) {
        let txq = {
            let s = self.lock().await;
            s.tx_queue.clone()
        };
        if let Some(q) = txq.get() {
            for c in data.bytes() {
                while q.push(c).is_err() {
                    executor::Task::yield_now().await;
                }
                self.enable_tx_interrupt().await;
            }
        }
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

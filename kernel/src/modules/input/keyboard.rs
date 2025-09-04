//! Covers functionality for keyboards

use crate::{Arc, IoPortRef, IoReadWrite, IrqGuardedInner, IrqGuardedSimple, IrqNumbers, Locked};
use core::task::Waker;

/// A stream struct for receiving serial data
struct Ps2InputStream {
    /// The data queue for the stream
    queue: Arc<IrqGuardedSimple<crossbeam::queue::ArrayQueue<u8>>>,
    /// The wakers for the stream
    wakers: Arc<IrqGuardedSimple<crossbeam::queue::ArrayQueue<Waker>>>,
}

impl futures::Stream for Ps2InputStream {
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

struct Registers {
    /// The data port for the controller
    data_port: IoPortRef<u8>,
    /// The status (read) and command (write) port for the controller
    status_command_port: IoPortRef<u8>,
}

/// THe inner struct for a ps2 struct
pub struct Ps2Inner {
    /// The registers for the controller
    registers: IrqGuardedSimple<Locked<Registers>>,
    /// The stream
    stream: crate::common::OneWayStream<u8>,
}

/// Ps2 hardware
pub struct Ps2 {
    /// The inner data
    inner: Arc<Ps2Inner>,
}

impl Ps2 {
    /// Create a new Self
    pub fn new() -> Option<Self> {
        let i = IrqGuardedInner::new(IrqNumbers::Only2([1, 12]), false, true, |_| {}, |_| {});
        let inner = Ps2Inner {
            registers: IrqGuardedSimple::new(
                Locked::new(Registers {
                    data_port: crate::IO_PORT_MANAGER?.get_port(0x60)?,
                    status_command_port: crate::IO_PORT_MANAGER?.get_port(0x64)?,
                }),
                &i,
            ),
            stream: crate::common::OneWayStream::new(&i, 30, 5),
        };
        let s = Self {
            inner: Arc::new(inner),
        };
        s.send_command(0xad);
        s.send_command(0xa7);
        s.read_buffer();
        s.send_command(0x20);
        let mut cc = s.read_buffer();
        cc |= 0x11;
        crate::VGA.print_str(&alloc::format!("KB CC {:x}\r\n", cc));
        s.send_command2(0x60, cc);
        s.send_command(0xaa);
        let check = s.read_response();
        if check != 0x55 {
            crate::VGA.print_str("Keyboard controller failed\r\n");
            loop {}
        }
        s.send_command(0xab);
        let test_ps2 = s.read_response();
        if test_ps2 != 0 {
            crate::VGA.print_str("Keyboard failed\r\n");
            loop {}
        }
        s.send_command(0xae);

        use crate::kernel::SystemTrait;
        let s2 = s.inner.clone();
        crate::SYSTEM
            .read()
            .register_irq_handler(1, move || Self::handle_interrupt(&s2));
        let s2 = s.inner.clone();
        crate::SYSTEM
            .read()
            .register_irq_handler(12, move || Self::handle_interrupt(&s2));

        Some(s)
    }

    /// Get a read stream for reading from the keyboard
    pub fn read_stream(&self) -> impl futures::Stream<Item = u8> {
        self.inner.stream.clone()
    }

    fn handle_interrupt(s: &Arc<Ps2Inner>) {
        use crate::common::IoReadWrite;
        let b = s
            .registers
            .interrupt_access()
            .sync_lock()
            .data_port
            .port_read();
        let _ = s.stream.push_interrupt(b);
    }

    fn read_buffer(&self) -> u8 {
        use crate::common::IoReadWrite;
        let p = self.inner.registers.access();
        let mut p = p.sync_lock();
        p.data_port.port_read()
    }

    /// Send a single byte command to the controller
    fn send_command(&self, cmd: u8) {
        use crate::common::IoReadWrite;
        self.inner
            .registers
            .access()
            .sync_lock()
            .status_command_port
            .port_write(cmd);
    }

    fn send_command2(&self, cmd1: u8, cmd2: u8) {
        use crate::common::IoReadWrite;
        let p = self.inner.registers.access();
        let mut p = p.sync_lock();
        p.status_command_port.port_write(cmd1);
        while (p.status_command_port.port_read() & 2) != 0 {}
        p.data_port.port_write(cmd2);
    }

    fn read_response(&self) -> u8 {
        use crate::common::IoReadWrite;
        let p = self.inner.registers.access();
        let mut p = p.sync_lock();
        while (p.status_command_port.port_read() & 1) == 0 {}
        p.data_port.port_read()
    }
}

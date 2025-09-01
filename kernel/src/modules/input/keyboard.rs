//! Covers functionality for keyboards

use crate::{Arc, IoPortRef, IoReadWrite, IrqGuardedInner, Locked};

struct Registers {
    /// The data port for the controller
    data_port: IoPortRef<u8>,
    /// The status (read) and command (write) port for the controller
    status_command_port: IoPortRef<u8>,
}

/// THe inner struct for a ps2 struct
pub struct Ps2Inner {
    /// The registers for the controller
    registers: crate::IrqGuardedSimple<Locked<Registers>>,
}

/// Ps2 hardware
pub struct Ps2 {
    /// The inner data
    inner: Arc<Ps2Inner>,
}

impl Ps2 {
    /// Create a new Self
    pub fn new() -> Option<Self> {
        let i = IrqGuardedInner::new(alloc::vec![1, 12], false, true, |_| {}, |_| {});
        let inner = Ps2Inner {
            registers: crate::IrqGuardedSimple::new(
                Locked::new(Registers {
                    data_port: crate::IO_PORT_MANAGER?.get_port(0x60)?,
                    status_command_port: crate::IO_PORT_MANAGER?.get_port(0x64)?,
                }),
                &i,
            ),
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

    fn handle_interrupt(s: &Arc<Ps2Inner>) {
        use crate::common::IoReadWrite;
        let b = s
            .registers
            .interrupt_access()
            .sync_lock()
            .data_port
            .port_read();
        crate::VGA.print_str(&alloc::format!("KEYBOARD {:X}\r\n", b));
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

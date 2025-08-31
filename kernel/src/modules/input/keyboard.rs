//! Covers functionality for keyboards

use crate::{Arc, IoPortRef, IrqGuardedInner, Locked};

/// THe inner struct for a ps2 struct
pub struct Ps2Inner {
    /// The data port for the controller
    data_port: crate::IrqGuardedSimple<Locked<IoPortRef<u8>>>,
    /// The status (read) and command (write) port for the controller
    status_command_port: crate::IrqGuardedSimple<Locked<IoPortRef<u8>>>,
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
            data_port: crate::IrqGuardedSimple::new(
                Locked::new(crate::IO_PORT_MANAGER?.get_port(0x60)?),
                &i,
            ),
            status_command_port: crate::IrqGuardedSimple::new(
                Locked::new(crate::IO_PORT_MANAGER?.get_port(0x64)?),
                &i,
            ),
        };
        let s = Self {
            inner: Arc::new(inner),
        };
        s.send_command(0xad);
        s.send_command(0xa7);
        s.read_buffer();
        
        Some(s)
    }

    fn read_buffer(&self) -> u8 {
        use crate::common::IoReadWrite;
        self.inner
            .data_port
            .access()
            .sync_lock()
            .port_read()
    }

    /// Send a single byte command to the controller
    fn send_command(&self, cmd: u8) -> Option<u8> {
        use crate::common::IoReadWrite;
        self.inner
            .status_command_port
            .access()
            .sync_lock()
            .port_write(cmd);
        None
    }
}

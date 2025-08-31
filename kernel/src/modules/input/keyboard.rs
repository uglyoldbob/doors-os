//! Covers functionality for keyboards

use crate::{Arc, IoPortRef};

/// THe inner struct for a ps2 struct
pub struct Ps2Inner {
    /// The data port for the controller
    data_port: crate::IrqGuardedSimple<IoPortRef<u8>>,
    /// The status (read) and command (write) port for the controller
    status_command_port: crate::IrqGuardedSimple<IoPortRef<u8>>,
}

/// Ps2 hardware
pub struct Ps2 {
    /// The inner data
    inner: Arc<Ps2Inner>,
}

impl Ps2 {
    /// Create a new Self
    pub fn new() -> Option<Self> {
        let inner = Ps2Inner {
            data_port: crate::IrqGuardedSimple::new(crate::IO_PORT_MANAGER?.get_port(0x60)?),
            status_command_port: crate::IrqGuardedSimple::new(crate::IO_PORT_MANAGER?.get_port(0x64)?),
        };
        let s = Self {
            inner: Arc::new(inner),
        };
        Some(s)
    }

    /// Send a single byte command to the controller
    fn send_command(&self, cmd: u8) -> Option<u8> {
        use crate::common::IoReadWrite;
        self.inner.status_command_port.port_write(cmd);
        None
    }
}
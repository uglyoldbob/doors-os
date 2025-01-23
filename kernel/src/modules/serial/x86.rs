//! Serial port code for x86 serial ports

use crate::boot::x86::IoPortArray;
use crate::boot::x86::IoReadWrite;
use crate::boot::x86::IOPORTS;
use crate::LockedArc;

/// A serial port (COM) for x86
pub struct X86SerialPort {
    /// The io ports
    base: IoPortArray<'static>,
}

impl X86SerialPort {
    /// Attempt to build a new serial port, probing it as needed
    pub fn new(base: u16) -> Option<Self> {
        let ports = IOPORTS.get_ports(base, 8).unwrap();
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
        ports.port(0).port_write(0xAAu8);
        let a: u8 = ports.port(0).port_read();
        if a == 0xaa {
            ports.port(4).port_write(0x03u8);
            Some(Self { base: ports })
        } else {
            None
        }
    }

    /// Check the transmit fifo to see if it is full
    fn can_send(&mut self) -> bool {
        let a: u8 = self.base.port(5).port_read();
        (a & 0x20) != 0
    }
}

impl super::SerialTrait for LockedArc<X86SerialPort> {
    fn setup(&self, rate: u32) -> Result<(), ()> {
        Err(())
    }

    fn sync_transmit(&self, data: &[u8]) {
        let mut s = self.lock();
        for c in data {
            while !s.can_send() {}
            s.base.port(0).port_write(*c);
        }
    }

    fn sync_transmit_str(&self, data: &str) {
        let mut s = self.lock();
        for c in data.bytes() {
            while !s.can_send() {}
            s.base.port(0).port_write(c);
        }
    }
}

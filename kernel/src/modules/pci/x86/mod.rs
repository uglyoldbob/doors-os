//! X86 pci bus code

use crate::boot::x86::{IoPortRef, IoReadWrite, IOPORTS};

use crate::modules::video::TextDisplayTrait;

use super::{ConfigurationSpace, INVALID_VENDOR};

struct PciDevice {
    dev: u8,
}

impl PciDevice {
    fn get_vendor(&self, pci: &mut Pci, bus: &PciBus) -> u16 {
        pci.read(bus.num, self.dev, 0, 0)
    }

    fn get_all_configuration(&self, pci: &mut Pci, bus: &PciBus) -> ConfigurationSpace {
        let mut s: [u32; 64] = [0; 64];
        for (i, v) in s.iter_mut().enumerate() {
            let low = pci.read(bus.num, self.dev, 0, i as u8 * 4);
            let high = pci.read(bus.num, self.dev, 0, i as u8 * 4 + 2);
            let combined: u32 = (low as u32) | (high as u32) << 16;
            *v = combined;
        }
        let a: super::ConfigurationSpaceC =
            unsafe { core::ptr::read_unaligned(s.as_ptr() as *const _) };
        a.unpack()
    }
}

struct PciBus {
    num: u8,
    devices: alloc::vec::Vec<PciDevice>,
}

impl PciBus {
    /// Probe the bus
    pub fn new(pci: &mut Pci, num: u8) -> Option<Self> {
        let mut found = false;
        let mut bus = PciBus {
            num,
            devices: alloc::vec::Vec::new(),
        };
        for dev in 0..32 {
            if let Some(d) = bus.find_device(pci, dev) {
                let conf = d.get_all_configuration(pci, &bus);
                doors_macros2::kernel_print!("\tFound device {}\r\n", dev);
                bus.devices.push(d);
                conf.dump();
                found = true;
            }
        }
        if found {
            Some(bus)
        } else {
            None
        }
    }

    /// Check to see if a specific device exists
    fn find_device(&self, pci: &mut Pci, devnum: u8) -> Option<PciDevice> {
        let d = PciDevice { dev: devnum };
        if d.get_vendor(pci, self) != INVALID_VENDOR {
            Some(d)
        } else {
            None
        }
    }
}

/// The x86 pci system instance
pub struct Pci {
    /// The address register
    address: IoPortRef<u32>,
    /// The data register
    data: IoPortRef<u32>,
    /// The busses
    busses: alloc::vec::Vec<PciBus>,
}

impl Pci {
    /// Attempt to construct a pci system
    pub fn new() -> Option<Self> {
        let pcia_address: IoPortRef<u32> = IOPORTS.get_port(0xcf8)?;
        let pcia_data: IoPortRef<u32> = IOPORTS.get_port(0xcfc)?;
        Some(Self {
            address: pcia_address,
            data: pcia_data,
            busses: alloc::vec::Vec::new(),
        })
    }
}

impl Pci {
    ///Read a configuration word
    fn read(&mut self, bus: u8, device: u8, function: u8, offset: u8) -> u16 {
        let a: u32 = ((bus as u32) << 16)
            | ((device as u32) << 11)
            | ((function as u32) << 8)
            | ((offset as u32) & 0xFC)
            | 0x8000_0000;
        self.address.port_write(a);
        let b: u32 = self.data.port_read();
        ((b >> ((offset & 2) << 3)) & 0xFFFF) as u16
    }
}

impl super::PciTrait for Pci {
    fn setup(&mut self) {
        doors_macros2::kernel_print!("pci: Probing for pci busses\r\n");
        for i in 0..=255 {
            if let Some(bus) = PciBus::new(self, i) {
                doors_macros2::kernel_print!("pci: Bus {} exists\r\n", i);
                self.busses.push(bus);
            }
        }
        doors_macros2::kernel_print!("pci: Done probing for pci busses\r\n");
    }
}

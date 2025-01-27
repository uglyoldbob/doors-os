//! X86 pci bus code

use crate::boot::x86::{IoPortRef, IoReadWrite, IOPORTS};

use crate::modules::video::TextDisplayTrait;

use super::{ConfigurationSpace, INVALID_VENDOR};

struct PciFunction {
    /// The pci function number
    function: u8,
}

impl PciFunction {
    /// Returns the vendor id by reading the value from pci configuration space
    /// function is specified by self
    /// device is specified by the parent PciDevice
    /// bus is specified by the grandparent PciBus
    /// configuration space is specified by Pci
    fn get_vendor(&self, pci: &mut PciRegisters, bus: &PciBus, dev: &PciDevice) -> u16 {
        pci.read(bus.num, dev.dev, self.function, 0)
    }

    /// Returns all configuratino space data, reading it from pci configuration space
    /// function is specified by self
    /// device is specified by the parent PciDevice
    /// bus is specified by the grandparent PciBus
    /// configuration space is specified by Pci
    fn get_all_configuration(
        &self,
        pci: &mut PciRegisters,
        bus: &PciBus,
        dev: &PciDevice,
    ) -> ConfigurationSpace {
        let mut s: [u32; 64] = [0; 64];
        for (i, v) in s.iter_mut().enumerate() {
            let low = pci.read(bus.num, dev.dev, 0, i as u8 * 4);
            let high = pci.read(bus.num, dev.dev, 0, i as u8 * 4 + 2);
            let combined: u32 = (low as u32) | (high as u32) << 16;
            *v = combined;
        }
        let a: super::ConfigurationSpaceC =
            unsafe { core::ptr::read_unaligned(s.as_ptr() as *const _) };
        a.unpack()
    }

    /// Returns true if the function header from the configuration space specifies multi-function
    fn is_multifunction(&self, pci: &mut PciRegisters, bus: &PciBus, dev: &PciDevice) -> bool {
        let bist_header: u16 = pci.read(bus.num, dev.dev, self.function, 14);
        let header: u8 = (bist_header & 0xFF) as u8;
        (header & 0x80) != 0
    }

    /// Print the details of this function
    fn print(&self, pci: &mut PciRegisters, bus: &PciBus, dev: &PciDevice) {
        let config = self.get_all_configuration(pci, bus, dev);
        config.dump("\t\t\t");
    }
}

struct PciDevice {
    /// The pci device number
    dev: u8,
    /// The functions available for this device
    functions: alloc::vec::Vec<PciFunction>,
}

impl PciDevice {
    /// Run a query to find all available functions and populate them for this device
    fn query_functions(mut self, pci: &mut PciRegisters, bus: &PciBus) -> Option<Self> {
        let f1 = PciFunction { function: 0 };
        if f1.get_vendor(pci, bus, &self) != INVALID_VENDOR {
            if f1.is_multifunction(pci, bus, &self) {
                for i in 1..8 {
                    let f = PciFunction { function: i };
                    if f.get_vendor(pci, bus, &self) != INVALID_VENDOR {
                        self.functions.push(f);
                    }
                }
            }
            self.functions.push(f1);
            Some(self)
        } else {
            None
        }
    }

    fn print_functions(&self, pci: &mut PciRegisters, bus: &PciBus) {
        for (i, f) in self.functions.iter().enumerate() {
            doors_macros2::kernel_print!("\t\tPCI Function {}\r\n", i);
            f.print(pci, bus, self);
        }
    }
}

struct PciBus {
    /// The pci bus number
    num: u8,
    /// The devices detected on the bus
    devices: alloc::vec::Vec<PciDevice>,
}

impl PciBus {
    /// Probe the bus
    pub fn new(pci: &mut PciRegisters, num: u8) -> Option<Self> {
        let mut found = false;
        let mut bus = PciBus {
            num,
            devices: alloc::vec::Vec::new(),
        };
        for dev in 0..32 {
            if let Some(d) = bus.find_device(pci, dev) {
                bus.devices.push(d);
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
    fn find_device(&self, pci: &mut PciRegisters, devnum: u8) -> Option<PciDevice> {
        let d = PciDevice {
            dev: devnum,
            functions: alloc::vec::Vec::new(),
        };
        d.query_functions(pci, self)
    }

    /// Print all devices on the bus
    fn print_devices(&self, pci: &mut PciRegisters) {
        for (i, d) in self.devices.iter().enumerate() {
            doors_macros2::kernel_print!("\tPCI device {}\r\n", i);
            d.print_functions(pci, self);
        }
    }
}

struct PciRegisters {
    /// The address register
    address: IoPortRef<u32>,
    /// The data register
    data: IoPortRef<u32>,
}

/// The x86 pci system instance
pub struct Pci {
    /// The io registers
    registers: PciRegisters,
    /// The busses
    busses: alloc::vec::Vec<PciBus>,
}

impl Pci {
    /// Attempt to construct a pci system
    pub fn new() -> Option<Self> {
        let pcia_address: IoPortRef<u32> = IOPORTS.get_port(0xcf8)?;
        let pcia_data: IoPortRef<u32> = IOPORTS.get_port(0xcfc)?;
        Some(Self {
            registers: PciRegisters {
                address: pcia_address,
                data: pcia_data,
            },
            busses: alloc::vec::Vec::new(),
        })
    }
}

impl PciRegisters {
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
            if let Some(bus) = PciBus::new(&mut self.registers, i) {
                doors_macros2::kernel_print!("pci: Bus {} exists\r\n", i);
                self.busses.push(bus);
            }
        }
        doors_macros2::kernel_print!("pci: Done probing for pci busses\r\n");
    }

    fn print_devices(&mut self) {
        for (i, bus) in self.busses.iter().enumerate() {
            doors_macros2::kernel_print!("PCI BUS {}\r\n", i);
            bus.print_devices(&mut self.registers);
        }
    }
}

//! X86 pci bus code

use alloc::format;

use crate::boot::x86::{IoPortRef, IOPORTS};
use crate::IoReadWrite;

/// Defines the io registers for x86 pci configuration space access
pub struct PciRegisters {
    /// The address register
    address: IoPortRef<u32>,
    /// The data register
    data: IoPortRef<u32>,
}

impl PciRegisters {
    /// Attempt to construct a new self
    pub fn new() -> Option<Self> {
        let pcia_address: IoPortRef<u32> = IOPORTS.get_port(0xcf8)?;
        let pcia_data: IoPortRef<u32> = IOPORTS.get_port(0xcfc)?;
        Some(Self {
            address: pcia_address,
            data: pcia_data,
        })
    }
}

impl PciRegisters {
    /// Set the pci configuration space address that will be either read or written next
    fn set_address(&mut self, bus: u8, device: u8, function: u8, offset: u8) {
        let a: u32 = ((bus as u32) << 16)
            | ((device as u32) << 11)
            | ((function as u32) << 8)
            | ((offset as u32) & 0xFC)
            | 0x8000_0000;
        self.address.port_write(a);
    }
}

impl super::PciConfigurationSpaceTrait for PciRegisters {
    fn read_u16(&mut self, bus: u8, device: u8, function: u8, offset: u8) -> u16 {
        self.set_address(bus, device, function, offset);
        let b: u32 = self.data.port_read();
        ((b >> ((offset & 2) << 3)) & 0xFFFF) as u16
    }

    fn read_u32(&mut self, bus: u8, device: u8, function: u8, offset: u8) -> u32 {
        self.set_address(bus, device, function, offset);
        self.data.port_read()
    }

    fn write_u32(&mut self, bus: u8, device: u8, function: u8, offset: u8, val: u32) {
        self.set_address(bus, device, function, offset);
        self.data.port_write(val);
    }
}

/// The x86 pci system instance
pub struct Pci {
    /// The configuration space access
    configuration: super::PciConfigurationSpace,
    /// The busses
    busses: alloc::vec::Vec<super::PciBus>,
}

impl Pci {
    /// Attempt to construct a pci system
    pub fn new() -> Option<Self> {
        Some(Self {
            configuration: super::PciConfigurationSpace::X86(PciRegisters::new()?),
            busses: alloc::vec::Vec::new(),
        })
    }
}

impl super::PciTrait for Pci {
    async fn setup(&mut self) {
        crate::VGA
            .print_str_async("pci: Probing for pci busses\r\n")
            .await;
        for i in 0..=255 {
            if let Some(bus) = super::PciBus::new(&mut self.configuration, i) {
                crate::VGA
                    .print_str_async(&format!("pci: Bus {} exists\r\n", i))
                    .await;
                self.busses.push(bus);
            }
        }
        crate::VGA
            .print_str_async("pci: Done probing for pci busses\r\n")
            .await;
    }

    async fn print_devices(&mut self) {
        for (i, bus) in self.busses.iter().enumerate() {
            crate::VGA
                .print_str_async(&format!("PCI BUS {}\r\n", i))
                .await;
            bus.print_devices(&mut self.configuration).await;
        }
    }

    async fn driver_run(
        &mut self,
        d: &mut alloc::collections::btree_map::BTreeMap<u32, super::PciFunctionDriver>,
    ) {
        for bus in &self.busses {
            bus.driver_run(d, &mut self.configuration).await;
        }
    }
}

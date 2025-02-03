//! Code for the pci bus

use crate::boot::x86::IoReadWrite;
use crate::modules::video::{hex_dump, hex_dump_generic, TextDisplayTrait};
use crate::{Locked, LockedArc};
use alloc::collections::BTreeMap;
use lazy_static::lazy_static;

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod x86;

lazy_static! {
    /// The entire list of gpios for the kernel
    pub static ref PCI_DRIVERS: LockedArc<BTreeMap<u32, PciFunctionDriver>> =
        LockedArc::new(BTreeMap::new());
}

/// Represents an invalid value for a pci vendor
const INVALID_VENDOR: u16 = 0xffff;

bitfield::bitfield! {
    struct DeviceControl(u16);
    impl Debug;
    impl new;
    /// Enables IO space for the device
    io_space, set_io_space: 0;
    /// Enables memory space access
    memory_space, set_memory_space: 1;
    /// Allows the device to act as a bus master
    bus_master, set_bus_master: 2;
    /// Enables the device to monitor special cycles
    special_cycles, set_special_cycles: 3;
    /// Set to allow the memory write and invalidate command
    mem_w_inval_enable, set_mem_w_inval_enable: 4;
    /// Enable palette snooping for vga palette registers
    vga_palette_snoop, set_vga_palette_snoop: 5;
    /// Control response to parity errors, enables PERR signal?
    parity_error, set_parity_error: 6;
    /// Enables address/data stepping
    stepping_control, set_stepping_control: 7;
    /// Enable the SERR# driver
    serr_n_enable, set_serr_n_enable: 8;
    /// Enable fast back-to-back transactions to different devices
    fast_back_back_enable, set_fast_back_back_enable: 9;
}

/// Represents the configuration space for a single device
pub struct ConfigurationSpaceStandard {
    /// The base address registers for the device
    bar: [u32; 6],
    /// Used for devices that do cardbus and pci (see pcmcia 2.0 for cis info)
    cardbus_cis: u32,
    /// Used to identify the expansion board or subsystem where the device exists
    subsystem_vendor: u16,
    /// Used to identify the expansion board or subsystem where the device exists
    subsystem: u16,
    /// Defines the base address and if the rom is enabled
    expansion_rom_base: u32,
    /// Points to a linked list of new capabilites implemented by this device
    capabilities_ptr: u8,
    /// Reserved
    _reserved1: [u8; 3],
    /// Reserved
    _reserved2: u32,
    /// Used to convey interrupt line routing information
    interrupt_line: u8,
    /// Defines which interrupt pin is used
    interrupt_pin: u8,
    /// Length of burst period needed for a 33mhz clock
    min_gnt: u8,
    /// Specifies how often the device needs to access the bus
    max_lat: u8,
    /// The rest of the header
    _remainder: [u32; 48],
}

impl From<[u32; 60]> for ConfigurationSpaceStandard {
    fn from(value: [u32; 60]) -> Self {
        let mut bar: [u32; 6] = [0; 6];
        for (i, b) in bar.iter_mut().enumerate() {
            *b = value[i];
        }
        let mut remainder: [u32; 48] = [0; 48];
        for (i, b) in remainder.iter_mut().enumerate() {
            *b = value[12 + i];
        }
        Self {
            bar,
            cardbus_cis: value[6],
            subsystem_vendor: (value[7] & 0xFFFF) as u16,
            subsystem: (value[7] >> 16) as u16,
            expansion_rom_base: value[8],
            capabilities_ptr: (value[9] & 0xff) as u8,
            _reserved1: [0; 3],
            _reserved2: 0,
            interrupt_line: (value[11] & 0xff) as u8,
            interrupt_pin: ((value[11] >> 8) & 0xff) as u8,
            min_gnt: ((value[11] >> 16) & 0xff) as u8,
            max_lat: ((value[11] >> 24) & 0xff) as u8,
            _remainder: remainder,
        }
    }
}

impl ConfigurationSpaceStandard {
    /// Dump the configuration data contents
    pub fn dump(&self, linestart: &str) {
        for i in 0..6 {
            doors_macros2::kernel_print!("{}BAR {}: {:X}\r\n", linestart, i, self.bar[i]);
        }
        doors_macros2::kernel_print!("{}Cardbus CIS {:x}\r\n", linestart, self.cardbus_cis);
        doors_macros2::kernel_print!(
            "{}Subsytem vendor {:x}\r\n",
            linestart,
            self.subsystem_vendor
        );
        doors_macros2::kernel_print!("{}Subsystem {:x}\r\n", linestart, self.subsystem);
        doors_macros2::kernel_print!(
            "{}Expansion rom {:x}\r\n",
            linestart,
            self.expansion_rom_base
        );
        doors_macros2::kernel_print!("{}Capabilites: {:x}\r\n", linestart, self.capabilities_ptr);
        doors_macros2::kernel_print!(
            "{}Interrupt line: {:X} pin {:X} \r\n",
            linestart,
            self.interrupt_line,
            self.interrupt_pin
        );
        doors_macros2::kernel_print!("{}Min gnt: {} \r\n", linestart, self.min_gnt);
        doors_macros2::kernel_print!("{}Max latency: {} \r\n", linestart, self.max_lat);
    }
}

/// Represents the configuration space for a single device
pub struct ConfigurationSpaceBridge {
    /// The base address registers for the device
    bar: [u32; 2],
    primary_bus: u8,
    secondary_bus: u8,
    subordinate_bus: u8,
    second_latency: u8,
    io_base: u8,
    io_limit: u8,
    status2: u16,
    memory_base: u16,
    memory_limit: u16,
    prefetchable_memory_base: u16,
    prefetchable_memory_limit: u16,
    prefetchable_base_upper: u32,
    prefetchable_limit_upper: u32,
    iobase_upper: u16,
    iolimit_upper: u16,
    /// Points to a linked list of new capabilites implemented by this device
    capabilities_ptr: u8,
    /// Reserved
    _reserved1: [u8; 3],
    /// Defines the base address and if the rom is enabled
    expansion_rom_base: u32,
    /// Used to convey interrupt line routing information
    interrupt_line: u8,
    /// Defines which interrupt pin is used
    interrupt_pin: u8,
    bridge_control: u16,
    /// The rest of the header
    _remainder: [u32; 48],
}

impl ConfigurationSpaceBridge {
    /// Dump the configuration data contents
    pub fn dump(&self, linestart: &str) {
        doors_macros2::kernel_print!("{}PCI Bridge\r\n", linestart);
        doors_macros2::kernel_print!("{}BAR: {:x} {:x}\r\n", linestart, self.bar[0], self.bar[1]);
        doors_macros2::kernel_print!(
            "{}Bus: {} {} {}\r\n",
            linestart,
            self.primary_bus,
            self.secondary_bus,
            self.subordinate_bus
        );
        doors_macros2::kernel_print!("{}Latency: {}\r\n", linestart, self.second_latency);
        doors_macros2::kernel_print!(
            "{}IO: {:x} size {:x}\r\n",
            linestart,
            self.io_base,
            self.io_limit
        );
        doors_macros2::kernel_print!("{}Status: {:x}\r\n", linestart, self.status2);
        doors_macros2::kernel_print!(
            "{}Memory: {:X} {:x}\r\n",
            linestart,
            self.memory_base,
            self.memory_limit
        );
        doors_macros2::kernel_print!(
            "{}Prefetchable: {:x} size {:x}\r\n",
            linestart,
            (self.prefetchable_base_upper as u64) << 32 | (self.prefetchable_memory_base as u64),
            (self.prefetchable_limit_upper as u64) << 32 | (self.prefetchable_memory_limit as u64),
        );
        doors_macros2::kernel_print!(
            "{}IO: {:x} size {:x}\r\n",
            linestart,
            (self.iobase_upper as u64) << 32 | (self.io_base as u64),
            (self.iolimit_upper as u64) << 32 | (self.io_limit as u64),
        );
        doors_macros2::kernel_print!("{}Capabilites: {:x}\r\n", linestart, self.capabilities_ptr);
        doors_macros2::kernel_print!(
            "{}Expansion rom: {:x}\r\n",
            linestart,
            self.expansion_rom_base
        );
        doors_macros2::kernel_print!(
            "{}Interrupt line: {:X} pin {:X} \r\n",
            linestart,
            self.interrupt_line,
            self.interrupt_pin
        );
        doors_macros2::kernel_print!("{}Bridge control: {:X}\r\n", linestart, self.bridge_control);
    }
}

impl From<[u32; 60]> for ConfigurationSpaceBridge {
    fn from(value: [u32; 60]) -> Self {
        let mut bar: [u32; 2] = [0; 2];
        for (i, b) in bar.iter_mut().enumerate() {
            *b = value[i];
        }
        let mut remainder: [u32; 48] = [0; 48];
        for (i, b) in remainder.iter_mut().enumerate() {
            *b = value[12 + i];
        }
        Self {
            bar,
            primary_bus: (value[2] & 0xFF) as u8,
            secondary_bus: ((value[2] >> 8) & 0xff) as u8,
            subordinate_bus: ((value[2] >> 16) & 0xff) as u8,
            second_latency: (value[2] >> 24) as u8,
            io_base: ((value[3]) & 0xff) as u8,
            io_limit: ((value[3] >> 8) & 0xff) as u8,
            status2: (value[3] >> 16) as u16,
            memory_base: ((value[4]) & 0xffff) as u16,
            memory_limit: (value[4] >> 16) as u16,
            prefetchable_memory_base: (value[5] & 0xFFFF) as u16,
            prefetchable_memory_limit: (value[5] >> 16) as u16,
            prefetchable_base_upper: value[6],
            prefetchable_limit_upper: value[7],
            iobase_upper: (value[8] & 0xFFFF) as u16,
            iolimit_upper: (value[8] >> 16) as u16,
            capabilities_ptr: (value[9] & 0xff) as u8,
            expansion_rom_base: value[10],
            _reserved1: [0; 3],
            interrupt_line: (value[11] & 0xff) as u8,
            interrupt_pin: ((value[11] >> 8) & 0xff) as u8,
            bridge_control: (value[11] >> 16) as u16,
            _remainder: remainder,
        }
    }
}

/// Represents the configuration space for a single device
pub struct ConfigurationSpaceCardbus {
    cardbus_base: u32,
    capabilities_offset: u8,
    _reserved: u8,
    status2: u16,
    pci_bus_num: u8,
    cardbus_bus_num: u8,
    subordinate_bus_num: u8,
    cardbus_latency: u8,
    memory_base0: u32,
    memory_limit0: u32,
    memory_base1: u32,
    memory_limit1: u32,
    io_base0: u32,
    io_limit0: u32,
    io_base1: u32,
    io_limit1: u32,
    /// Used to convey interrupt line routing information
    interrupt_line: u8,
    /// Defines which interrupt pin is used
    interrupt_pin: u8,
    bridge_control: u16,
    legacy_base_addr: u32,
    /// The rest of the header
    _remainder: [u32; 47],
}

impl ConfigurationSpaceCardbus {
    /// Dump the configuration data contents
    pub fn dump(&self, linestart: &str) {
        doors_macros2::kernel_print!("{}CARDBUS Device\r\n", linestart);
        doors_macros2::kernel_print!("{}Base: {:X}\r\n", linestart, self.cardbus_base);
        doors_macros2::kernel_print!("{}Offset: {:X}\r\n", linestart, self.capabilities_offset);
        doors_macros2::kernel_print!("{}Status2: {:X}\r\n", linestart, self.status2);
        doors_macros2::kernel_print!(
            "{}Pci: {}, cardbus {}, sub {}\r\n",
            linestart,
            self.pci_bus_num,
            self.cardbus_bus_num,
            self.subordinate_bus_num
        );
        doors_macros2::kernel_print!("{}Latency: {}\r\n", linestart, self.cardbus_latency);
        doors_macros2::kernel_print!(
            "{}Memory0: {:X} size {:x}\r\n",
            linestart,
            self.memory_base0,
            self.memory_limit0
        );
        doors_macros2::kernel_print!(
            "{}Memory1: {:X} size {:x}\r\n",
            linestart,
            self.memory_base1,
            self.memory_limit1
        );
        doors_macros2::kernel_print!(
            "{}IO0: {:X} size {:x}\r\n",
            linestart,
            self.io_base0,
            self.io_limit0
        );
        doors_macros2::kernel_print!(
            "{}IO1: {:X} size {:x}\r\n",
            linestart,
            self.io_base1,
            self.io_limit1
        );
        doors_macros2::kernel_print!(
            "{}Interrupt line: {:X} pin {:X} \r\n",
            linestart,
            self.interrupt_line,
            self.interrupt_pin
        );
        doors_macros2::kernel_print!("{}Bridge control: {:X}\r\n", linestart, self.bridge_control);
        doors_macros2::kernel_print!("{}Legacy base: {:X}\r\n", linestart, self.legacy_base_addr);
    }
}

impl From<[u32; 60]> for ConfigurationSpaceCardbus {
    fn from(value: [u32; 60]) -> Self {
        let mut remainder: [u32; 47] = [0; 47];
        for (i, b) in remainder.iter_mut().enumerate() {
            *b = value[12 + i];
        }
        Self {
            cardbus_base: value[0],
            capabilities_offset: (value[1] & 0xFF) as u8,
            _reserved: 0,
            status2: (value[1] >> 16) as u16,
            pci_bus_num: (value[2] & 0xFF) as u8,
            cardbus_bus_num: ((value[2] >> 8) & 0xFF) as u8,
            subordinate_bus_num: ((value[2] >> 16) & 0xFF) as u8,
            cardbus_latency: ((value[2] >> 24) & 0xFF) as u8,
            memory_base0: value[3],
            memory_limit0: value[4],
            memory_base1: value[5],
            memory_limit1: value[6],
            io_base0: value[7],
            io_limit0: value[8],
            io_base1: value[9],
            io_limit1: value[10],
            interrupt_line: (value[11] & 0xff) as u8,
            interrupt_pin: ((value[11] >> 8) & 0xff) as u8,
            bridge_control: (value[11] >> 16) as u16,
            legacy_base_addr: value[12],
            _remainder: remainder,
        }
    }
}

/// The configuration space data that changes on a function by function basis
#[repr(C)]
pub enum ConfigurationSpaceEnum {
    /// configuration for a standard function
    Standard(ConfigurationSpaceStandard),
    /// configuration for a pci bridge
    Bridge(ConfigurationSpaceBridge),
    /// configuration for a cardbus device
    Cardbus(ConfigurationSpaceCardbus),
}

impl ConfigurationSpaceEnum {
    /// Return an iterator over the bar offsets (in bytes) from the start of configuration space
    pub fn get_bars(&self) -> &[u8] {
        match self {
            ConfigurationSpaceEnum::Standard(_configuration_space_standard) => {
                &[16, 20, 24, 28, 32, 36]
            }
            ConfigurationSpaceEnum::Bridge(_configuration_space_bridge) => &[16, 20],
            ConfigurationSpaceEnum::Cardbus(_configuration_space_cardbus) => &[],
        }
    }
}

const _CONFIGURATION_SPACE_CHECKER: [u8; 256] = [0; core::mem::size_of::<ConfigurationSpace>()];

#[repr(C, packed)]
struct ConfigurationSpaceC {
    /// The manufacturer of the device
    vendor: u16,
    /// The device id, assigned by the vendor
    device: u16,
    /// Controls how the device responds to and generates pci cycles
    command: DeviceControl,
    /// Reports status information related to pci bus events
    status: u16,
    /// Device revision defined by vendor
    revision: u8,
    /// Defines (if applicable) the register level programming interface)
    prog_if: u8,
    /// Identifies the functino of the device
    subclass: u8,
    /// Defines a generic function of the device
    class: u8,
    /// The size of the cacheline in u32 units.
    cache_size: u8,
    /// The number of pci clocks for the latency timer
    latency: u8,
    /// Defines the layout of the header starting at bar
    header: u8,
    /// Built in self test control and status
    bist: u8,
    /// The rest of the header
    remainder: [u32; 60],
}

impl ConfigurationSpaceC {
    /// Unpack the structure to a more convenient form
    pub fn unpack(&self) -> ConfigurationSpace {
        ConfigurationSpace {
            vendor: self.vendor,
            device: self.device,
            command: DeviceControl(self.command.0),
            status: self.status,
            revision: self.revision,
            prog_if: self.prog_if,
            subclass: self.subclass,
            class: self.class,
            cache_size: self.cache_size,
            latency: self.latency,
            header: self.header,
            bist: self.bist,
            remainder: self.remainder,
        }
    }
}

struct ConfigurationSpace {
    /// The manufacturer of the device
    vendor: u16,
    /// The device id, assigned by the vendor
    device: u16,
    /// Controls how the device responds to and generates pci cycles
    command: DeviceControl,
    /// Reports status information related to pci bus events
    status: u16,
    /// Device revision defined by vendor
    revision: u8,
    /// Defines (if applicable) the register level programming interface)
    prog_if: u8,
    /// Identifies the functino of the device
    subclass: u8,
    /// Defines a generic function of the device
    class: u8,
    /// The size of the cacheline in u32 units.
    cache_size: u8,
    /// The number of pci clocks for the latency timer
    latency: u8,
    /// Defines the layout of the header starting at bar
    header: u8,
    /// Built in self test control and status
    bist: u8,
    /// The rest of the header
    remainder: [u32; 60],
}

impl Default for ConfigurationSpace {
    fn default() -> Self {
        Self {
            vendor: INVALID_VENDOR,
            device: 0,
            command: DeviceControl(0),
            status: 0,
            revision: 0,
            prog_if: 0,
            subclass: 0,
            class: 0,
            cache_size: 0,
            latency: 0,
            header: 0,
            bist: 0,
            remainder: [0; 60],
        }
    }
}

impl ConfigurationSpace {
    /// Dump the configuration space
    pub fn dump(&self, linestart: &str) {
        doors_macros2::kernel_print!("{}Configuration space:\r\n", linestart);
        doors_macros2::kernel_print!("{}Vendor: {:x}\r\n", linestart, self.vendor);
        doors_macros2::kernel_print!("{}Device: {:x}\r\n", linestart, self.device);
        doors_macros2::kernel_print!("{}Command: {:x}\r\n", linestart, self.command.0);
        doors_macros2::kernel_print!("{}Status: {:x}\r\n", linestart, self.status);
        doors_macros2::kernel_print!("{}Revision: {:x}\r\n", linestart, self.revision);
        doors_macros2::kernel_print!("{}ProgIf: {:x}\r\n", linestart, self.prog_if);
        doors_macros2::kernel_print!("{}Subclass: {:x}\r\n", linestart, self.subclass);
        doors_macros2::kernel_print!("{}Class: {:x}\r\n", linestart, self.class);
        doors_macros2::kernel_print!("{}Cache: {:x}\r\n", linestart, self.cache_size);
        doors_macros2::kernel_print!("{}Latency: {:x}\r\n", linestart, self.latency);
        doors_macros2::kernel_print!("{}HEADER: {:x}\r\n", linestart, self.header);
        doors_macros2::kernel_print!("{}BIST: {:x}\r\n", linestart, self.bist);
        if let Some(h) = self.get_space() {
            match h {
                ConfigurationSpaceEnum::Standard(cs) => {
                    cs.dump(linestart);
                }
                ConfigurationSpaceEnum::Bridge(cs) => {
                    cs.dump(linestart);
                }
                ConfigurationSpaceEnum::Cardbus(cs) => {
                    cs.dump(linestart);
                }
            }
        }
    }

    /// Try to get the configuration space
    pub fn get_space(&self) -> Option<ConfigurationSpaceEnum> {
        match self.header & 0x7f {
            0 => Some(ConfigurationSpaceEnum::Standard(self.remainder.into())),
            1 => Some(ConfigurationSpaceEnum::Bridge(self.remainder.into())),
            2 => Some(ConfigurationSpaceEnum::Cardbus(self.remainder.into())),
            _ => None,
        }
    }

    /// Return an iterator over the bar offsets
    /// The FnMut accepts an index, a bar offset and a second bar offset for 64-bit bars, returns true if the bar was 64-bits
    fn process_bars(
        &self,
        bars: &mut [Option<BarSpace>],
        mut f: impl FnMut(u8, u8, Option<u8>) -> BarSpace,
    ) {
        for b in bars.iter_mut() {
            *b = None;
        }
        let mut index = 0;
        if let Some(space) = self.get_space() {
            let bars_indexes = space.get_bars();
            let mut skip = false;
            let mut iter = bars_indexes.iter();
            let first_bar = iter.next();
            if let Some(fbar) = first_bar {
                let mut prev_bar = *fbar;
                let mut prev_index = 0;
                let mut last_bar = *fbar;
                for (i, bar) in iter.enumerate() {
                    let i = i + 1;
                    last_bar = *bar;
                    if !skip {
                        let newbar = f(prev_index as u8, prev_bar, Some(*bar));
                        bars[index] = Some(newbar);
                        index += 1;
                        skip = newbar.is64();
                    } else {
                        skip = false;
                    }
                    prev_index = i;
                    prev_bar = *bar;
                }
                let newbar = f(prev_index as u8, last_bar, None);
                bars[index] = Some(newbar);
                index += 1;
            }
        }
    }
}

/// The pci system trait
#[enum_dispatch::enum_dispatch]
pub trait PciTrait {
    /// Setup the pci system
    fn setup(&mut self);
    /// Print all devices on the system
    fn print_devices(&mut self);
    /// Run all drivers that can be associated with pci functions
    fn driver_run(
        &mut self,
        system: &mut impl crate::kernel::SystemTrait,
        d: &mut BTreeMap<u32, PciFunctionDriver>,
    );
}

/// A BAR space
#[derive(Clone, Copy)]
pub enum BarSpace {
    /// A memory space, 32 bits wide
    Memory32 {
        /// The base address
        base: u32,
        /// The size in bytes
        size: u32,
        /// The flags for the space
        flags: u8,
        /// The index for the bar space
        index: u8,
    },
    /// A memory space, 64 bits wide
    Memory64 {
        /// The base address
        base: u64,
        /// The size in bytes
        size: u64,
        /// The flags for the space
        flags: u8,
        /// The index for the bar space
        index: u8,
    },
    /// IO space
    IO {
        /// The base address
        base: u32,
        /// The size in bytes
        size: u32,
        /// The index for the bar space
        index: u8,
    },
    /// Memory space not configured or is invalid
    Invalid {
        /// The index for the bar space
        index: u8,
    },
}

impl Default for BarSpace {
    fn default() -> Self {
        Self::Invalid { index: 0 }
    }
}

impl BarSpace {
    /// Is the space a 64-bit space?
    pub fn is64(&self) -> bool {
        if let Self::Memory64 {
            base: _,
            size: _,
            flags: _,
            index: _,
        } = self
        {
            true
        } else {
            false
        }
    }

    /// Is the space valid (is the bar size non-zero)?
    fn is_size_valid(&self) -> bool {
        if let Self::Invalid { index: _ } = self {
            false
        } else {
            true
        }
    }

    /// Returns the bar space index
    fn get_index(&self) -> u8 {
        match self {
            BarSpace::Memory32 {
                base: _,
                size: _,
                flags: _,
                index,
            } => *index,
            BarSpace::Memory64 {
                base: _,
                size: _,
                flags: _,
                index,
            } => *index,
            BarSpace::IO {
                base: _,
                size: _,
                index,
            } => *index,
            BarSpace::Invalid { index } => *index,
        }
    }

    /// Obtain the io space specified by the bar, only if it is io space and an io manager exists
    fn get_io(
        &mut self,
        system: &mut impl crate::kernel::SystemTrait,
        pci: &mut PciConfigurationSpace,
        bus: &PciBus,
        dev: &PciDevice,
        function: &PciFunction,
        config: &ConfigurationSpaceEnum,
    ) -> Option<crate::IoPortArray<'static>> {
        if let Self::IO { base, size, index } = self {
            if let Some(iom) = crate::IO_PORT_MANAGER {
                if *base == 0 {
                    panic!("Unable to assign io port ranges yet");
                }
                let ports = iom.get_ports(*base as u16, *size as u16);
                if let Some(p) = ports {
                    if *base == 0 {
                        let addr = p.get_base() as u32;
                        let newbar = BarSpace::IO {
                            base: addr,
                            size: *size,
                            index: *index,
                        };
                        doors_macros2::kernel_print!("Writing bar with address {:x}\r\n", addr);
                        newbar.write_to_pci(pci, bus, dev, function, config);
                        *self = newbar;
                    }
                    Some(p)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Obtain the memory space specified by the bar, only if it is memory space
    fn get_memory<'b>(
        &mut self,
        system: &mut impl crate::kernel::SystemTrait,
        pci: &mut PciConfigurationSpace,
        bus: &PciBus,
        dev: &PciDevice,
        function: &PciFunction,
        config: &ConfigurationSpaceEnum,
    ) -> Option<crate::PciMemory> {
        match self {
            BarSpace::Memory32 {
                base: _,
                size,
                flags,
                index,
            } => {
                let pcim = crate::PciMemory::new(*size as usize);
                if let Ok(pcim) = &pcim {
                    let newbar = BarSpace::Memory32 {
                        base: pcim.phys as u32,
                        size: pcim.size as u32,
                        flags: *flags,
                        index: *index,
                    };
                    doors_macros2::kernel_print!("Writing bar with address {:x}\r\n", pcim.phys);
                    newbar.write_to_pci(pci, bus, dev, function, config);
                    *self = newbar;
                }
                pcim.ok()
            }
            BarSpace::Memory64 {
                base: _,
                size: _,
                flags: _,
                index: _,
            } => {
                todo!()
            }
            BarSpace::IO {
                base: _,
                size: _,
                index: _,
            } => None,
            BarSpace::Invalid { index: _ } => None,
        }
    }

    /// Write the bar to the pci configuration space
    fn write_to_pci(
        &self,
        pci: &mut PciConfigurationSpace,
        bus: &PciBus,
        dev: &PciDevice,
        function: &PciFunction,
        config: &ConfigurationSpaceEnum,
    ) {
        let bar = config.get_bars();
        match self {
            BarSpace::Memory32 {
                base,
                size: _,
                flags,
                index,
            } => {
                pci.write_u32(
                    bus.num,
                    dev.dev,
                    function.function,
                    bar[*index as usize],
                    *base | (*flags as u32),
                );
            }
            BarSpace::Memory64 {
                base,
                size: _,
                flags,
                index,
            } => {
                let lv: u32 = (*base & 0xFFFFFFFF) as u32 | (*flags as u32);
                pci.write_u32(
                    bus.num,
                    dev.dev,
                    function.function,
                    bar[*index as usize],
                    lv,
                );
                let hv = (*base >> 32) as u32;
                pci.write_u32(
                    bus.num,
                    dev.dev,
                    function.function,
                    bar[1 + *index as usize],
                    hv,
                );
            }
            BarSpace::IO {
                base,
                size: _,
                index,
            } => {
                pci.write_u32(
                    bus.num,
                    dev.dev,
                    function.function,
                    bar[*index as usize],
                    *base | 1,
                );
            }
            BarSpace::Invalid { index } => {
                pci.write_u32(bus.num, dev.dev, function.function, bar[*index as usize], 0);
            }
        }
    }

    fn print(&self) {
        match self {
            BarSpace::Memory32 {
                base,
                size,
                flags,
                index: _,
            } => {
                doors_macros2::kernel_print!(
                    "BAR32: {:x} x {:x} flags {:x}\r\n",
                    base,
                    size,
                    flags
                );
            }
            BarSpace::Memory64 {
                base,
                size,
                flags,
                index: _,
            } => {
                doors_macros2::kernel_print!(
                    "BAR64: {:x} x {:x} flags {:x}\r\n",
                    base,
                    size,
                    flags
                );
            }
            BarSpace::IO {
                base,
                size,
                index: _,
            } => {
                doors_macros2::kernel_print!("BARIO: {:x} x {:x}\r\n", base, size);
            }
            BarSpace::Invalid { index: _ } => {
                doors_macros2::kernel_print!("BAR INVALID\r\n");
            }
        }
    }
}

/// A single function of a single or multi-function pci device
pub struct PciFunction {
    /// The pci function number
    function: u8,
    /// The configuration data
    configuration: Option<ConfigurationSpace>,
}

impl PciFunction {
    /// Construct a new pci function
    pub fn new(function: u8) -> Self {
        Self {
            function,
            configuration: None,
        }
    }

    /// Returns a combination of vendor and device id, to identify a potential driver for the function
    fn get_driver_id(&self, pci: &mut PciConfigurationSpace, bus: &PciBus, dev: &PciDevice) -> u32 {
        pci.read_u32(bus.num, dev.dev, self.function, 0)
    }

    /// Returns the vendor id by reading the value from pci configuration space
    /// function is specified by self
    /// device is specified by the parent PciDevice
    /// bus is specified by the grandparent PciBus
    /// configuration space is specified by Pci
    fn get_vendor(&self, pci: &mut PciConfigurationSpace, bus: &PciBus, dev: &PciDevice) -> u16 {
        pci.read_u16(bus.num, dev.dev, self.function, 0)
    }

    /// Returns all configuration space data, reading it from pci configuration space
    /// function is specified by self
    /// device is specified by the parent PciDevice
    /// bus is specified by the grandparent PciBus
    /// configuration space is specified by Pci
    fn get_all_configuration(
        &self,
        pci: &mut PciConfigurationSpace,
        bus: &PciBus,
        dev: &PciDevice,
    ) -> ConfigurationSpace {
        let mut s: [u32; 64] = [0; 64];
        for (i, v) in s.iter_mut().enumerate() {
            *v = pci.read_u32(bus.num, dev.dev, self.function, i as u8 * 4);
        }
        let a: ConfigurationSpaceC = unsafe { core::ptr::read_unaligned(s.as_ptr() as *const _) };
        a.unpack()
    }

    /// Returns true if the function header from the configuration space specifies multi-function
    fn is_multifunction(
        &self,
        pci: &mut PciConfigurationSpace,
        bus: &PciBus,
        dev: &PciDevice,
    ) -> bool {
        let bist_header: u16 = pci.read_u16(bus.num, dev.dev, self.function, 14);
        let header: u8 = (bist_header & 0xFF) as u8;
        (header & 0x80) != 0
    }

    /// Parse the bar registers for the function
    fn parse_bars(
        &self,
        bars: &mut [Option<BarSpace>; 6],
        pci: &mut PciConfigurationSpace,
        bus: &PciBus,
        dev: &PciDevice,
        config: &ConfigurationSpace,
    ) {
        pci.write_u32(
            bus.num,
            dev.dev,
            self.function,
            4,
            (config.status as u32) << 16 | config.command.0 as u32 & 0xFFFC,
        );
        config.process_bars(bars, |barnum, bar, bar64| {
            let orig_bar: u32 = pci.read_u32(bus.num, dev.dev, self.function, bar);
            let mut upper_orig_bar: u32 = 0;

            if (orig_bar & 4) != 0 {
                if let Some(b) = bar64 {
                    upper_orig_bar = pci.read_u32(bus.num, dev.dev, self.function, b);
                } else {
                    panic!("Cannot pull second bar for 64-bits");
                }
            }

            pci.write_u32(bus.num, dev.dev, self.function, bar, 0xFFFFFFFF);
            if (orig_bar & 4) != 0 {
                pci.write_u32(bus.num, dev.dev, self.function, bar64.unwrap(), 0xFFFFFFFF);
            }
            let size = pci.read_u32(bus.num, dev.dev, self.function, bar);
            let barspace = if size & 1 == 0 {
                //memory space
                let bar = if (orig_bar & 4) != 0 {
                    let usize: u32 = pci.read_u32(bus.num, dev.dev, self.function, bar);
                    let bar64 = orig_bar as u64;
                    let size64 = (size as u64) | (usize as u64) << 32;
                    let size: u64 = !(size64 & 0xFFFFFFFFFFFFFFF0) + 1;
                    if size64 != 0 {
                        BarSpace::Memory64 {
                            base: bar64 & 0xFFFFFFFFFFFFFFF0,
                            size,
                            flags: (orig_bar & 0xF) as u8,
                            index: barnum,
                        }
                    } else {
                        BarSpace::Invalid { index: barnum }
                    }
                } else {
                    let size: u32 = !(size & 0xFFFFFFF0) + 1;
                    if size != 0 {
                        BarSpace::Memory32 {
                            base: orig_bar & 0xFFFFFFF0,
                            size,
                            flags: (orig_bar & 0xF) as u8,
                            index: barnum,
                        }
                    } else {
                        BarSpace::Invalid { index: barnum }
                    }
                };
                if (orig_bar & 4) != 0 {
                    pci.write_u32(
                        bus.num,
                        dev.dev,
                        self.function,
                        bar64.unwrap(),
                        upper_orig_bar,
                    );
                }
                bar
            } else {
                //io space
                let size: u32 = !(size & 0xFFFFFFFC) + 1;
                let bar = BarSpace::IO {
                    base: orig_bar & 0xFFFFFFFC,
                    size,
                    index: barnum,
                };
                bar
            };
            pci.write_u32(bus.num, dev.dev, self.function, bar, orig_bar);
            barspace
        });
        pci.write_u32(
            bus.num,
            dev.dev,
            self.function,
            4,
            (config.status as u32) << 16 | config.command.0 as u32,
        );
    }

    /// Print the details of this function
    fn print(&self, pci: &mut PciConfigurationSpace, bus: &PciBus, dev: &PciDevice) {
        let config = self.get_all_configuration(pci, bus, dev);
        config.dump("\t\t\t");
    }
}

/// A single pci device, with one or more functions
pub struct PciDevice {
    /// The pci device number
    dev: u8,
    /// The functions available for this device
    functions: alloc::vec::Vec<PciFunction>,
}

impl PciDevice {
    /// Run a query to find all available functions and populate them for this device
    fn query_functions(mut self, pci: &mut PciConfigurationSpace, bus: &PciBus) -> Option<Self> {
        let f1 = PciFunction::new(0);
        if f1.get_vendor(pci, bus, &self) != INVALID_VENDOR {
            if f1.is_multifunction(pci, bus, &self) {
                for i in 1..8 {
                    let f = PciFunction::new(i);
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

    fn print_functions(&self, pci: &mut PciConfigurationSpace, bus: &PciBus) {
        for (i, f) in self.functions.iter().enumerate() {
            doors_macros2::kernel_print!("\t\tPCI Function {}\r\n", i);
            f.print(pci, bus, self);
        }
    }
}

/// Represents a single pci bus
pub struct PciBus {
    /// The pci bus number
    num: u8,
    /// The devices detected on the bus
    devices: alloc::vec::Vec<PciDevice>,
}

impl PciBus {
    /// Probe the bus
    pub fn new(pci: &mut PciConfigurationSpace, num: u8) -> Option<Self> {
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
    fn find_device(&self, pci: &mut PciConfigurationSpace, devnum: u8) -> Option<PciDevice> {
        let d = PciDevice {
            dev: devnum,
            functions: alloc::vec::Vec::new(),
        };
        d.query_functions(pci, self)
    }

    /// Print all devices on the bus
    fn print_devices(&self, pci: &mut PciConfigurationSpace) {
        for (i, d) in self.devices.iter().enumerate() {
            doors_macros2::kernel_print!("\tPCI device {}\r\n", i);
            d.print_functions(pci, self);
        }
    }

    /// Run drivers that can be associated with pci functions
    fn driver_run(
        &self,
        system: &mut impl crate::kernel::SystemTrait,
        map: &mut alloc::collections::btree_map::BTreeMap<u32, PciFunctionDriver>,
        pci: &mut PciConfigurationSpace,
    ) {
        for d in &self.devices {
            for f in &d.functions {
                let id = f.get_driver_id(pci, self, d);
                doors_macros2::kernel_print!("Checking pci device {:x}\r\n", id);
                if map.contains_key(&id) {
                    let config = f.get_all_configuration(pci, self, d);
                    let code = map.get_mut(&id).unwrap();
                    let mut bars: [Option<BarSpace>; 6] = [None; 6];
                    f.parse_bars(&mut bars, pci, self, d, &config);
                    code.parse_bars(system, pci, self, d, f, &config.get_space().unwrap(), bars);
                } else {
                    doors_macros2::kernel_print!("Unknown PCI FUNCTION: {:X}\r\n", id);
                    let config = f.get_all_configuration(pci, self, d);
                    config.dump("\t");
                }
            }
        }
    }
}

/// a pci bus instance
#[enum_dispatch::enum_dispatch(PciTrait)]
pub enum Pci {
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    /// X86 pci bus
    X86Pci(x86::Pci),
}

impl Pci {
    /// Run all drivers for this pci system
    pub fn driver_setup(&mut self, system: &mut impl crate::kernel::SystemTrait) {
        let mut d = PCI_DRIVERS.lock();
        self.driver_run(system, &mut d);
    }
}

/// The trait for accessing pci configuration space
#[enum_dispatch::enum_dispatch]
trait PciConfigurationSpaceTrait {
    /// Read a configuration word
    fn read_u16(&mut self, bus: u8, device: u8, function: u8, offset: u8) -> u16;
    /// Read a configuration dword
    fn read_u32(&mut self, bus: u8, device: u8, function: u8, offset: u8) -> u32;
    /// Write a configuration dword
    fn write_u32(&mut self, bus: u8, device: u8, function: u8, offset: u8, val: u32);
}

/// The enum for accessing pci configuration space
#[enum_dispatch::enum_dispatch(PciConfigurationSpaceTrait)]
pub enum PciConfigurationSpace {
    /// Access pci configuration space with io on x86
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    X86(x86::PciRegisters),
}

/// The trait that pci function drivers must implement
#[enum_dispatch::enum_dispatch]
pub trait PciFunctionDriverTrait: Clone + Default {
    /// Register the driver in the given map, must check to see if the driver is already registered
    fn register(&self, m: &mut BTreeMap<u32, PciFunctionDriver>);

    /// Parse a bar register for the device
    fn parse_bars(
        &mut self,
        system: &mut impl crate::kernel::SystemTrait,
        cs: &mut PciConfigurationSpace,
        bus: &PciBus,
        dev: &PciDevice,
        f: &PciFunction,
        config: &ConfigurationSpaceEnum,
        bars: [Option<BarSpace>; 6],
    );
}

/// Register all pci drivers with the driver map
pub fn pci_register_drivers() {
    let mut drivers = PCI_DRIVERS.lock();

    doors_macros2::kernel_print!("Registering pci drivers\r\n");
    for d in PCI_CODE {
        d.register(&mut drivers);
    }
    doors_macros2::kernel_print!("Done registering pci drivers\r\n");
}

/// Represents a device driver for a pci function
#[enum_dispatch::enum_dispatch(PciFunctionDriverTrait)]
#[derive(Clone)]
pub enum PciFunctionDriver {
    /// A dummy driver so the enum isn't empty
    Dummy(DummyPciFunctionDriver),
    /// Intel pro1000 ethernet driver
    IntelPro1000(IntelPro1000),
}

impl Default for PciFunctionDriver {
    fn default() -> Self {
        Self::Dummy(DummyPciFunctionDriver::default())
    }
}

/// Holds the pci drivers so that they can register with the `PCI_DRIVERS` variable
static PCI_CODE: &[PciFunctionDriver] = &[
    PciFunctionDriver::Dummy(DummyPciFunctionDriver {}),
    PciFunctionDriver::IntelPro1000(IntelPro1000::new()),
];

/// A dummy pci driver that does nothing
#[derive(Clone, Default)]
pub struct DummyPciFunctionDriver {}

impl PciFunctionDriverTrait for DummyPciFunctionDriver {
    fn register(&self, m: &mut BTreeMap<u32, PciFunctionDriver>) {
        doors_macros2::kernel_print!("Register dummy pci driver\r\n");
    }

    fn parse_bars(
        &mut self,
        _system: &mut impl crate::kernel::SystemTrait,
        _cs: &mut PciConfigurationSpace,
        _bus: &PciBus,
        _dev: &PciDevice,
        _f: &PciFunction,
        _config: &ConfigurationSpaceEnum,
        _bars: [Option<BarSpace>; 6],
    ) {
        panic!();
    }
}

/// Holds either memory or io space
enum MemoryOrIo {
    Memory(crate::PciMemory),
    Io(crate::IoPortArray<'static>),
}

impl MemoryOrIo {
    fn hex_dump(&self) {
        match self {
            MemoryOrIo::Memory(m) => {
                let mut buffer = [0u32; 32];
                for (i, b) in buffer.iter_mut().enumerate() {
                    *b = self.read(i as u16);
                }
                hex_dump_generic(&buffer, true, false);
            }
            MemoryOrIo::Io(io_port_array) => todo!(),
        }
    }

    fn read(&self, address: u16) -> u32 {
        match self {
            MemoryOrIo::Memory(mem) => mem.read_u32(address as usize),
            MemoryOrIo::Io(io) => {
                let mut iop: crate::IoPortRef<u32> = io.port(address);
                iop.port_read()
            }
        }
    }

    fn write(&mut self, address: u16, val: u32) {
        match self {
            MemoryOrIo::Memory(mem) => {
                mem.write_u32(address as usize, val);
            }
            MemoryOrIo::Io(io) => {
                let mut iop: crate::IoPortRef<u32> = io.port(address);
                iop.port_write(val);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u16)]
enum IntelPro1000Registers {
    Eeprom = 0x14,
    Rctrl = 0x100,
    Tctrl = 0x400,
    RxDescLow = 0x2800,
    RxDescHigh = 0x2804,
    RxDescLen = 0x2808,
    RxDescHead = 0x2810,
    RxDescTail = 0x2818,
    TxDescLow = 0x3800,
    TxDescHigh = 0x3804,
    TxDescLen = 0x3808,
    TxDescHead = 0x3810,
    TxDescTail = 0x3818,
}

/// Ethernet driver for the intel pro/1000 ethernet controller on pci
/// TODO: move this to crate::modules::network
#[derive(Clone, Default)]
pub struct IntelPro1000 {}

struct MacAddress {
    address: [u8; 6],
}

#[repr(C, packed)]
struct RxBuffer {
    address: u64,
    length: u16,
    checksum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

impl RxBuffer {
    fn new() -> Self {
        let buf: alloc::boxed::Box<[u8; 8192]> = alloc::boxed::Box::new([0; 8192]);
        let buf2 = alloc::boxed::Box::leak(buf);
        Self {
            address: crate::slice_address(buf2) as u64,
            length: 0,
            checksum: 0,
            status: 0,
            errors: 0,
            special: 0,
        }
    }
}

#[repr(C, packed)]
struct TxBuffer {
    address: u64,
    length: u16,
    cso: u8,
    cmd: u8,
    status: u8,
    css: u8,
    special: u16,
}

impl TxBuffer {
    fn new() -> Self {
        let buf: alloc::boxed::Box<[u8; 8192]> = alloc::boxed::Box::new([0; 8192]);
        let buf2 = alloc::boxed::Box::leak(buf);
        Self {
            address: crate::slice_address(buf2) as u64,
            length: 0,
            cso: 0,
            cmd: 0,
            status: 0,
            css: 1,
            special: 0,
        }
    }
}

/// Holds all information required for the multiple rx buffers required for the network card
struct RxBuffers {
    /// The structures used by the network card
    bufs: crate::DmaMemorySlice<RxBuffer>,
    /// The structures used to manage the buffers
    dmas: alloc::vec::Vec<crate::DmaMemorySlice<u8>>,
}

impl RxBuffers {
    fn new(quantity: u8, size: usize) -> Result<Self, core::alloc::AllocError> {
        let m: crate::DmaMemorySlice<RxBuffer> =
            crate::DmaMemorySlice::new_with(quantity as usize, |_| Ok(RxBuffer::new()))?;
        let mut dmas = alloc::vec::Vec::with_capacity(quantity as usize);
        for i in 0..quantity {
            dmas.push(crate::DmaMemorySlice::new(size)?);
        }
        Ok(Self { bufs: m, dmas })
    }
}

/// Holds all information required for the multiple tx buffers required for the network card
struct TxBuffers {
    /// The structures used by the network card
    bufs: crate::DmaMemorySlice<TxBuffer>,
    /// The structures used to manage the buffers
    dmas: alloc::vec::Vec<crate::DmaMemorySlice<u8>>,
}

impl TxBuffers {
    fn new(quantity: u8, size: usize) -> Result<Self, core::alloc::AllocError> {
        let m: crate::DmaMemorySlice<TxBuffer> =
            crate::DmaMemorySlice::new_with(quantity as usize, |_| Ok(TxBuffer::new()))?;
        let mut dmas = alloc::vec::Vec::with_capacity(quantity as usize);
        for i in 0..quantity {
            dmas.push(crate::DmaMemorySlice::new(size)?);
        }
        Ok(Self { bufs: m, dmas })
    }
}

struct IntelPro1000Device {
    /// The base address registers
    bars: [Option<BarSpace>; 6],
    /// The memory allocated by bar0
    bar0: MemoryOrIo,
    /// the io space allocated for the device
    io: crate::IoPortArray<'static>,
    /// Is the eeprom present?
    eeprom_present: Option<bool>,
    /// The rx buffers
    rxbufs: Option<RxBuffers>,
    /// The current rx buffer
    rxbufindex: Option<u8>,
    /// The tx buffers
    txbufs: Option<TxBuffers>,
    /// The current tx buffer
    txbufindex: Option<u8>,
}

const RCTRL_EN: u32 = 1 << 1;
const RCTRL_SBP: u32 = 1 << 2;
const RCTRL_UPE: u32 = 1 << 3;
const RCTRL_MPE: u32 = 1 << 4;
const RCTRL_LBM_NONE: u32 = 0;
const RCTRL_RDMTS_HALF: u32 = 0;
const RCTRL_BAM: u32 = 1 << 15;
const RCTRL_SECRC: u32 = 1 << 26;
const RCTRL_BSIZE_8192: u32 = 2 << 16 | 1 << 25;

const TCTRL_EN: u32 = 1 << 1;
const TCTRL_PSP: u32 = 1 << 3;
const TCTRL_CT_SHIFT: u8 = 4;
const TCTRL_COLD_SHIFT: u8 = 12u8;
const TCTRL_RTLC: u32 = 1 << 24;

impl IntelPro1000Device {
    fn detect_eeprom(&mut self) -> bool {
        if self.eeprom_present.is_none() {
            self.bar0.write(IntelPro1000Registers::Eeprom as u16, 1);
            self.eeprom_present = Some(false);
            for _i in 0..10000 {
                let val = self.bar0.read(IntelPro1000Registers::Eeprom as u16);
                let val2 = val & 0x10;
                doors_macros2::kernel_print!("EEPROM DETECT: {:x} {:x}\r\n", val, val2);
                if (val2) != 0 {
                    self.eeprom_present = Some(true);
                    break;
                }
            }
        }
        self.eeprom_present.unwrap()
    }

    fn init_rx(&mut self) -> Result<(), core::alloc::AllocError> {
        if self.rxbufs.is_none() {
            doors_macros2::kernel_print!("A\r\n");
            let rxbuf = RxBuffers::new(32, 8192)?;
            doors_macros2::kernel_print!("B\r\n");
            let rxaddr = rxbuf.bufs.phys;
            doors_macros2::kernel_print!("Writing RX stuff to network card\r\n");
            self.bar0.write(
                IntelPro1000Registers::RxDescLow as u16,
                (rxaddr >> 32) as u32,
            );
            self.bar0.write(
                IntelPro1000Registers::RxDescHigh as u16,
                (rxaddr & 0xFFFFFFFF) as u32,
            );
            self.bar0.write(
                IntelPro1000Registers::RxDescLen as u16,
                core::mem::size_of::<RxBuffer>() as u32 * rxbuf.bufs.len() as u32,
            );
            self.bar0.write(IntelPro1000Registers::RxDescHead as u16, 0);
            self.bar0.write(
                IntelPro1000Registers::RxDescTail as u16,
                rxbuf.bufs.len() as u32 - 1,
            );
            self.bar0.write(
                IntelPro1000Registers::Rctrl as u16,
                RCTRL_EN
                    | RCTRL_SBP
                    | RCTRL_UPE
                    | RCTRL_MPE
                    | RCTRL_LBM_NONE
                    | RCTRL_RDMTS_HALF
                    | RCTRL_BAM
                    | RCTRL_SECRC
                    | RCTRL_BSIZE_8192,
            );
            self.rxbufindex = Some(0);
            doors_macros2::kernel_print!("C\r\n");
            doors_macros2::kernel_print!("RX BUFFER ARRAY IS AT {:x}\r\n", rxaddr);
            for r in rxbuf.bufs.iter() {
                doors_macros2::kernel_print!("\tIndividual buffer addr is {:x}\r\n", unsafe {
                    core::ptr::read_unaligned(&raw const r.address)
                });
            }
            self.rxbufs = Some(rxbuf);
        }
        Ok(())
    }

    fn init_tx(&mut self) -> Result<(), core::alloc::AllocError> {
        if self.txbufs.is_none() {
            let txbuf = TxBuffers::new(8, 8192)?;
            let txaddr = txbuf.bufs.phys;
            self.bar0.write(
                IntelPro1000Registers::TxDescLow as u16,
                (txaddr >> 32) as u32,
            );
            self.bar0.write(
                IntelPro1000Registers::TxDescHigh as u16,
                (txaddr & 0xFFFFFFFF) as u32,
            );
            self.bar0.write(
                IntelPro1000Registers::TxDescLen as u16,
                core::mem::size_of::<TxBuffer>() as u32 * txbuf.bufs.len() as u32,
            );
            self.bar0.write(IntelPro1000Registers::TxDescHead as u16, 0);
            self.bar0.write(IntelPro1000Registers::TxDescTail as u16, 0);
            self.bar0.write(
                IntelPro1000Registers::Tctrl as u16,
                TCTRL_EN
                    | TCTRL_PSP
                    | (15 << TCTRL_CT_SHIFT)
                    | (64 << TCTRL_COLD_SHIFT)
                    | TCTRL_RTLC,
            );
            self.txbufindex = Some(0);
            doors_macros2::kernel_print!("TX BUFFER ARRAY IS AT {:x}\r\n", txaddr);
            for t in txbuf.bufs.iter() {
                doors_macros2::kernel_print!("\tIndividual buffer addr is {:x}\r\n", unsafe {
                    core::ptr::read_unaligned(&raw const t.address)
                });
            }
            self.txbufs = Some(txbuf);
        }
        Ok(())
    }

    fn get_mac_address(&mut self) -> MacAddress {
        if self.detect_eeprom() {
            let v = self.read_from_eeprom(0);
            let v2 = self.read_from_eeprom(1);
            let v3 = self.read_from_eeprom(2);
            let v = v.to_le_bytes();
            let v2 = v2.to_le_bytes();
            let v3 = v3.to_le_bytes();
            MacAddress {
                address: [v[0], v[1], v2[0], v2[1], v3[0], v3[1]],
            }
        } else {
            todo!();
        }
    }

    fn read_from_eeprom(&mut self, addr: u8) -> u16 {
        if self.detect_eeprom() {
            self.bar0.write(
                IntelPro1000Registers::Eeprom as u16,
                1 | ((addr as u32) << 8),
            );
            loop {
                let a = self.bar0.read(IntelPro1000Registers::Eeprom as u16);
                if (a & (0x10)) != 0 {
                    return (a >> 16) as u16;
                } else {
                    //doors_macros2::kernel_print!("VAL1: {:x}\r\n", a);
                }
            }
        } else {
            self.bar0.write(
                IntelPro1000Registers::Eeprom as u16,
                1 | ((addr as u32) << 2),
            );
            loop {
                let a = self.bar0.read(IntelPro1000Registers::Eeprom as u16);
                if (a & (0x2)) != 0 {
                    return (a >> 16) as u16;
                } else {
                    //doors_macros2::kernel_print!("VAL2: {:x}\r\n", a);
                }
            }
        }
    }
}

impl IntelPro1000 {
    /// Create a new self, in const form
    pub const fn new() -> Self {
        Self {}
    }
}

impl PciFunctionDriverTrait for IntelPro1000 {
    fn register(&self, m: &mut BTreeMap<u32, PciFunctionDriver>) {
        if !m.contains_key(&0x100e8086) {
            doors_macros2::kernel_print!("Register intel pro/1000 pci driver\r\n");
            m.insert(0x100e8086, self.clone().into());
        }
    }

    fn parse_bars(
        &mut self,
        system: &mut impl crate::kernel::SystemTrait,
        cs: &mut PciConfigurationSpace,
        bus: &PciBus,
        dev: &PciDevice,
        f: &PciFunction,
        config: &ConfigurationSpaceEnum,
        mut bars: [Option<BarSpace>; 6],
    ) {
        let bar0 = {
            if let Some(bar) = &mut bars[0] {
                if bar.is_size_valid() {
                    doors_macros2::kernel_print!("PCI PARSE BAR {}\r\n", bar.get_index());
                    bar.print();
                    let d = bar.get_memory(system, cs, bus, dev, f, config);
                    if let Some(d) = d {
                        doors_macros2::kernel_print!("Got memory at {:x}\r\n", d.virt);
                        Some(MemoryOrIo::Memory(d))
                    } else {
                        todo!();
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };
        let io = bars.iter_mut().find_map(|a| {
            if let Some(a) = a {
                a.get_io(system, cs, bus, dev, f, config)
            } else {
                None
            }
        });
        let configspace = f.get_all_configuration(cs, bus, dev);
        configspace.dump("\t");
        if let Some(m) = bar0 {
            if let Some(i) = io {
                for b in &bars {
                    if let Some(b) = b {
                        b.print();
                    }
                }
                let mut d = IntelPro1000Device {
                    bars,
                    bar0: m,
                    io: i,
                    eeprom_present: None,
                    rxbufs: None,
                    rxbufindex: None,
                    txbufs: None,
                    txbufindex: None,
                };
                d.bar0.hex_dump();
                doors_macros2::kernel_print!("EEPROM DETECTED: {}\r\n", d.detect_eeprom());
                let mut data = [0u16; 256];
                for (i, data) in data.iter_mut().enumerate() {
                    *data = d.read_from_eeprom(i as u8);
                }
                hex_dump_generic(&data, true, false);
                hex_dump(&d.get_mac_address().address, false, false);
                if let Err(e) = d.init_rx() {
                    doors_macros2::kernel_print!("RX buffer allocation error {:?}\r\n", e);
                }
                if let Err(e) = d.init_tx() {
                    doors_macros2::kernel_print!("TX buffer allocation error {:?}\r\n", e);
                }
            }
        }
    }
}

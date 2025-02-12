//! Code for the pci bus

use crate::LockedArc;
use alloc::{collections::BTreeMap, format};
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
    pub async fn dump(&self, linestart: &str) {
        for i in 0..6 {
            crate::VGA
                .print_str_async(&format!("{}BAR {}: {:X}\r\n", linestart, i, self.bar[i]))
                .await;
        }
        crate::VGA
            .print_str_async(&format!(
                "{}Cardbus CIS {:x}\r\n",
                linestart, self.cardbus_cis
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Subsytem vendor {:x}\r\n",
                linestart, self.subsystem_vendor
            ))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Subsystem {:x}\r\n", linestart, self.subsystem))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Expansion rom {:x}\r\n",
                linestart, self.expansion_rom_base
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Capabilites: {:x}\r\n",
                linestart, self.capabilities_ptr
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Interrupt line: {:X} pin {:X} \r\n",
                linestart, self.interrupt_line, self.interrupt_pin
            ))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Min gnt: {} \r\n", linestart, self.min_gnt))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Max latency: {} \r\n", linestart, self.max_lat))
            .await;
    }
}

/// Represents the configuration space for a single device
pub struct ConfigurationSpaceBridge {
    /// The base address registers for the device
    bar: [u32; 2],
    /// TODO
    primary_bus: u8,
    /// TODO
    secondary_bus: u8,
    /// TODO
    subordinate_bus: u8,
    /// TODO
    second_latency: u8,
    /// TODO
    io_base: u8,
    /// TODO
    io_limit: u8,
    /// TODO
    status2: u16,
    /// TODO
    memory_base: u16,
    /// TODO
    memory_limit: u16,
    /// TODO
    prefetchable_memory_base: u16,
    /// TODO
    prefetchable_memory_limit: u16,
    /// TODO
    prefetchable_base_upper: u32,
    /// TODO
    prefetchable_limit_upper: u32,
    /// TODO
    iobase_upper: u16,
    /// TODO
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
    /// TODO
    bridge_control: u16,
    /// The rest of the header
    _remainder: [u32; 48],
}

impl ConfigurationSpaceBridge {
    /// Dump the configuration data contents
    pub async fn dump(&self, linestart: &str) {
        crate::VGA
            .print_str_async(&format!("{}PCI Bridge\r\n", linestart))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}BAR: {:x} {:x}\r\n",
                linestart, self.bar[0], self.bar[1]
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Bus: {} {} {}\r\n",
                linestart, self.primary_bus, self.secondary_bus, self.subordinate_bus
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Latency: {}\r\n",
                linestart, self.second_latency
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}IO: {:x} size {:x}\r\n",
                linestart, self.io_base, self.io_limit
            ))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Status: {:x}\r\n", linestart, self.status2))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Memory: {:X} {:x}\r\n",
                linestart, self.memory_base, self.memory_limit
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Prefetchable: {:x} size {:x}\r\n",
                linestart,
                ((self.prefetchable_base_upper as u64) << 32)
                    | (self.prefetchable_memory_base as u64),
                ((self.prefetchable_limit_upper as u64) << 32)
                    | (self.prefetchable_memory_limit as u64),
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}IO: {:x} size {:x}\r\n",
                linestart,
                ((self.iobase_upper as u64) << 32) | (self.io_base as u64),
                ((self.iolimit_upper as u64) << 32) | (self.io_limit as u64),
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Capabilites: {:x}\r\n",
                linestart, self.capabilities_ptr
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Expansion rom: {:x}\r\n",
                linestart, self.expansion_rom_base
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Interrupt line: {:X} pin {:X} \r\n",
                linestart, self.interrupt_line, self.interrupt_pin
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Bridge control: {:X}\r\n",
                linestart, self.bridge_control
            ))
            .await;
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
    /// TODO
    cardbus_base: u32,
    /// TODO
    capabilities_offset: u8,
    /// Reserved
    _reserved: u8,
    /// TODO
    status2: u16,
    /// TODO
    pci_bus_num: u8,
    /// TODO
    cardbus_bus_num: u8,
    /// TODO
    subordinate_bus_num: u8,
    /// TODO
    cardbus_latency: u8,
    /// TODO
    memory_base0: u32,
    /// TODO
    memory_limit0: u32,
    /// TODO
    memory_base1: u32,
    /// TODO
    memory_limit1: u32,
    /// TODO
    io_base0: u32,
    /// TODO
    io_limit0: u32,
    /// TODO
    io_base1: u32,
    /// TODO
    io_limit1: u32,
    /// Used to convey interrupt line routing information
    interrupt_line: u8,
    /// Defines which interrupt pin is used
    interrupt_pin: u8,
    /// TODO
    bridge_control: u16,
    /// TODO
    legacy_base_addr: u32,
    /// The rest of the header
    _remainder: [u32; 47],
}

impl ConfigurationSpaceCardbus {
    /// Dump the configuration data contents
    pub async fn dump(&self, linestart: &str) {
        crate::VGA
            .print_str_async(&format!("{}CARDBUS Device\r\n", linestart))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Base: {:X}\r\n", linestart, self.cardbus_base))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Offset: {:X}\r\n",
                linestart, self.capabilities_offset
            ))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Status2: {:X}\r\n", linestart, self.status2))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Pci: {}, cardbus {}, sub {}\r\n",
                linestart, self.pci_bus_num, self.cardbus_bus_num, self.subordinate_bus_num
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Latency: {}\r\n",
                linestart, self.cardbus_latency
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Memory0: {:X} size {:x}\r\n",
                linestart, self.memory_base0, self.memory_limit0
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Memory1: {:X} size {:x}\r\n",
                linestart, self.memory_base1, self.memory_limit1
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}IO0: {:X} size {:x}\r\n",
                linestart, self.io_base0, self.io_limit0
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}IO1: {:X} size {:x}\r\n",
                linestart, self.io_base1, self.io_limit1
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Interrupt line: {:X} pin {:X} \r\n",
                linestart, self.interrupt_line, self.interrupt_pin
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Bridge control: {:X}\r\n",
                linestart, self.bridge_control
            ))
            .await;
        crate::VGA
            .print_str_async(&format!(
                "{}Legacy base: {:X}\r\n",
                linestart, self.legacy_base_addr
            ))
            .await;
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

/// Verifies the size of [ConfigurationSpace] is correct
const _CONFIGURATION_SPACE_CHECKER: [u8; 256] = [0; core::mem::size_of::<ConfigurationSpace>()];

/// A packed version of pci configuration space, see [ConfigurationSpace] and [ConfigurationSpaceC::unpack] for a more friendly version
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

/// Stores the pci configuration data for a pci function
pub struct ConfigurationSpace {
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
    /// Retrieve the device id for the function
    pub fn get_device_id(&self) -> u16 {
        self.device
    }

    /// Dump the configuration space
    pub async fn dump(&self, linestart: &str) {
        crate::VGA
            .print_str_async(&format!("{}Configuration space:\r\n", linestart))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Vendor: {:x}\r\n", linestart, self.vendor))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Device: {:x}\r\n", linestart, self.device))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Command: {:x}\r\n", linestart, self.command.0))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Status: {:x}\r\n", linestart, self.status))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Revision: {:x}\r\n", linestart, self.revision))
            .await;
        crate::VGA
            .print_str_async(&format!("{}ProgIf: {:x}\r\n", linestart, self.prog_if))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Subclass: {:x}\r\n", linestart, self.subclass))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Class: {:x}\r\n", linestart, self.class))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Cache: {:x}\r\n", linestart, self.cache_size))
            .await;
        crate::VGA
            .print_str_async(&format!("{}Latency: {:x}\r\n", linestart, self.latency))
            .await;
        crate::VGA
            .print_str_async(&format!("{}HEADER: {:x}\r\n", linestart, self.header))
            .await;
        crate::VGA
            .print_str_async(&format!("{}BIST: {:x}\r\n", linestart, self.bist))
            .await;
        if let Some(h) = self.get_space() {
            match h {
                ConfigurationSpaceEnum::Standard(cs) => {
                    cs.dump(linestart).await;
                }
                ConfigurationSpaceEnum::Bridge(cs) => {
                    cs.dump(linestart).await;
                }
                ConfigurationSpaceEnum::Cardbus(cs) => {
                    cs.dump(linestart).await;
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
            }
        }
    }
}

/// The pci system trait
#[enum_dispatch::enum_dispatch]
pub trait PciTrait {
    /// Setup the pci system
    async fn setup(&mut self);
    /// Print all devices on the system
    async fn print_devices(&mut self);
    /// Run all drivers that can be associated with pci functions
    async fn driver_run(&mut self, d: &mut BTreeMap<u32, PciFunctionDriver>);
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
        matches!(
            self,
            Self::Memory64 {
                base: _,
                size: _,
                flags: _,
                index: _,
            }
        )
    }

    /// Is the space valid (is the bar size non-zero)?
    pub fn is_size_valid(&self) -> bool {
        !matches!(self, Self::Invalid { index: _ })
    }

    /// Returns the bar space index
    pub fn get_index(&self) -> u8 {
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
    pub fn get_io(
        &mut self,
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
                        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                            "Writing bar with address {:x}\r\n",
                            addr
                        ));
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
    pub fn get_memory(
        &mut self,
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
                        base: pcim.phys() as u32,
                        size: pcim.size() as u32,
                        flags: *flags,
                        index: *index,
                    };
                    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                        "Writing bar with address {:x}\r\n",
                        pcim.phys()
                    ));
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

    /// Print the bar for analysis
    pub fn print(&self) {
        match self {
            BarSpace::Memory32 {
                base,
                size,
                flags,
                index: _,
            } => {
                crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                    "BAR32: {:x} x {:x} flags {:x}\r\n",
                    base,
                    size,
                    flags
                ));
            }
            BarSpace::Memory64 {
                base,
                size,
                flags,
                index: _,
            } => {
                crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                    "BAR64: {:x} x {:x} flags {:x}\r\n",
                    base,
                    size,
                    flags
                ));
            }
            BarSpace::IO {
                base,
                size,
                index: _,
            } => {
                crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                    "BARIO: {:x} x {:x}\r\n",
                    base,
                    size
                ));
            }
            BarSpace::Invalid { index: _ } => {
                crate::VGA.print_str("BAR INVALID\r\n");
            }
        }
    }
}

/// A single function of a single or multi-function pci device
pub struct PciFunction {
    /// The pci function number
    function: u8,
    /// The configuration data
    _configuration: Option<ConfigurationSpace>,
}

impl PciFunction {
    /// Construct a new pci function
    pub fn new(function: u8) -> Self {
        Self {
            function,
            _configuration: None,
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
    pub fn get_all_configuration(
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
            ((config.status as u32) << 16) | config.command.0 as u32 & 0xFFFC,
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
                    let size64 = (size as u64) | ((usize as u64) << 32);
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
                BarSpace::IO {
                    base: orig_bar & 0xFFFFFFFC,
                    size,
                    index: barnum,
                }
            };
            pci.write_u32(bus.num, dev.dev, self.function, bar, orig_bar);
            barspace
        });
        pci.write_u32(
            bus.num,
            dev.dev,
            self.function,
            4,
            ((config.status as u32) << 16) | config.command.0 as u32,
        );
    }

    /// Print the details of this function
    async fn print(&self, pci: &mut PciConfigurationSpace, bus: &PciBus, dev: &PciDevice) {
        let config = self.get_all_configuration(pci, bus, dev);
        config.dump("\t\t\t").await;
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

    /// Print all the functions of the device
    async fn print_functions(&self, pci: &mut PciConfigurationSpace, bus: &PciBus) {
        for (i, f) in self.functions.iter().enumerate() {
            crate::VGA
                .print_str_async(&format!("\t\tPCI Function {}\r\n", i))
                .await;
            f.print(pci, bus, self).await;
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
    async fn print_devices(&self, pci: &mut PciConfigurationSpace) {
        for (i, d) in self.devices.iter().enumerate() {
            crate::VGA
                .print_str_async(&format!("\tPCI device {}\r\n", i))
                .await;
            d.print_functions(pci, self).await;
        }
    }

    /// Run drivers that can be associated with pci functions
    async fn driver_run(
        &self,
        map: &mut alloc::collections::btree_map::BTreeMap<u32, PciFunctionDriver>,
        pci: &mut PciConfigurationSpace,
    ) {
        for d in &self.devices {
            for f in &d.functions {
                let id = f.get_driver_id(pci, self, d);
                crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                    "Checking pci device {:x}\r\n",
                    id
                ));
                if map.contains_key(&id) {
                    let config = f.get_all_configuration(pci, self, d);
                    let code = map.get_mut(&id).unwrap();
                    let mut bars: [Option<BarSpace>; 6] = [None; 6];
                    f.parse_bars(&mut bars, pci, self, d, &config);
                    code.parse_bars(pci, self, d, f, &config.get_space().unwrap(), bars);
                } else {
                    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                        "Unknown PCI FUNCTION: {:X}\r\n",
                        id
                    ));
                    let config = f.get_all_configuration(pci, self, d);
                    config.dump("\t").await;
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
    pub async fn driver_setup(&mut self) {
        let mut d = PCI_DRIVERS.lock();
        self.driver_run(&mut d).await;
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

    crate::VGA.print_str("Registering pci drivers\r\n");
    for d in PCI_CODE {
        d.register(&mut drivers);
    }
    crate::VGA.print_str("Done registering pci drivers\r\n");
}

/// Represents a device driver for a pci function
#[enum_dispatch::enum_dispatch(PciFunctionDriverTrait)]
#[derive(Clone)]
pub enum PciFunctionDriver {
    /// A dummy driver so the enum isn't empty
    Dummy(DummyPciFunctionDriver),
    /// Intel pro1000 ethernet driver
    IntelPro1000(crate::modules::network::intel::IntelPro1000),
}

impl Default for PciFunctionDriver {
    fn default() -> Self {
        Self::Dummy(DummyPciFunctionDriver::default())
    }
}

/// Holds the pci drivers so that they can register with the `PCI_DRIVERS` variable
static PCI_CODE: &[PciFunctionDriver] = &[
    PciFunctionDriver::Dummy(DummyPciFunctionDriver {}),
    PciFunctionDriver::IntelPro1000(crate::modules::network::intel::IntelPro1000::new()),
];

/// A dummy pci driver that does nothing
#[derive(Clone, Default)]
pub struct DummyPciFunctionDriver {}

impl PciFunctionDriverTrait for DummyPciFunctionDriver {
    fn register(&self, _m: &mut BTreeMap<u32, PciFunctionDriver>) {
        crate::VGA.print_str("Register dummy pci driver\r\n");
    }

    fn parse_bars(
        &mut self,
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

/// Setup the x86 pci space and register all pci drivers
pub async fn setup_pci() {
    let pci = crate::modules::pci::x86::Pci::new();
    if let Some(pci) = pci {
        let mut pci = crate::modules::pci::Pci::X86Pci(pci);
        pci.setup().await;
        crate::modules::pci::pci_register_drivers();
        pci.driver_setup().await;
    }
}

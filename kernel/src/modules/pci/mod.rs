//! Code for the pci bus

use crate::modules::video::TextDisplayTrait;
use crate::LockedArc;
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
struct ConfigurationSpaceStandard {
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
    reserved1: [u8; 3],
    /// Reserved
    reserved2: u32,
    /// Used to convey interrupt line routing information
    interrupt_line: u8,
    /// Defines which interrupt pin is used
    interrupt_pin: u8,
    /// Length of burst period needed for a 33mhz clock
    min_gnt: u8,
    /// Specifies how often the device needs to access the bus
    max_lat: u8,
    /// The rest of the header
    remainder: [u32; 48],
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
            reserved1: [0; 3],
            reserved2: 0,
            interrupt_line: (value[11] & 0xff) as u8,
            interrupt_pin: ((value[11] >> 8) & 0xff) as u8,
            min_gnt: ((value[11] >> 16) & 0xff) as u8,
            max_lat: ((value[11] >> 24) & 0xff) as u8,
            remainder,
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
struct ConfigurationSpaceBridge {
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
    reserved1: [u8; 3],
    /// Defines the base address and if the rom is enabled
    expansion_rom_base: u32,
    /// Used to convey interrupt line routing information
    interrupt_line: u8,
    /// Defines which interrupt pin is used
    interrupt_pin: u8,
    bridge_control: u16,
    /// The rest of the header
    remainder: [u32; 48],
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
            reserved1: [0; 3],
            interrupt_line: (value[11] & 0xff) as u8,
            interrupt_pin: ((value[11] >> 8) & 0xff) as u8,
            bridge_control: (value[11] >> 16) as u16,
            remainder,
        }
    }
}

/// Represents the configuration space for a single device
struct ConfigurationSpaceCardbus {
    cardbus_base: u32,
    capabilities_offset: u8,
    reserved: u8,
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
    remainder: [u32; 47],
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
            reserved: 0,
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
            remainder,
        }
    }
}

#[repr(C)]
enum ConfigurationSpaceEnum {
    Standard(ConfigurationSpaceStandard),
    Bridge(ConfigurationSpaceBridge),
    Cardbus(ConfigurationSpaceCardbus),
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

    /// Get the vendor register
    pub fn get_vendor(&self) -> u16 {
        self.vendor
    }

    /// Get the device id
    pub fn get_device_id(&self) -> u16 {
        self.device
    }

    /// Get the revision id
    pub fn get_revision_id(&self) -> u8 {
        self.revision
    }

    /// Is the device multi-function?
    pub fn is_multi_function(&self) -> bool {
        (self.header & 0x80) != 0
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
}

/// The pci system trait
#[enum_dispatch::enum_dispatch]
pub trait PciTrait {
    /// Setup the pci system
    fn setup(&mut self);
    /// Print all devices on the system
    fn print_devices(&mut self);
    /// Run all drivers that can be associated with pci functions
    fn driver_run(&mut self, d: &mut BTreeMap<u32, PciFunctionDriver>);
}

/// A single function of a single or multi-function pci device
pub struct PciFunction {
    /// The pci function number
    function: u8,
}

impl PciFunction {
    /// Returns a combination of vendor and device id, to identify a potential driver for the function
    fn get_driver_id(&self, pci: &mut PciConfigurationSpace, bus: &PciBus, dev: &PciDevice) -> u32 {
        (pci.read(bus.num, dev.dev, self.function, 0) as u32) << 16
            | pci.read(bus.num, dev.dev, self.function, 2) as u32
    }

    /// Returns the vendor id by reading the value from pci configuration space
    /// function is specified by self
    /// device is specified by the parent PciDevice
    /// bus is specified by the grandparent PciBus
    /// configuration space is specified by Pci
    fn get_vendor(&self, pci: &mut PciConfigurationSpace, bus: &PciBus, dev: &PciDevice) -> u16 {
        pci.read(bus.num, dev.dev, self.function, 0)
    }

    /// Returns all configuratino space data, reading it from pci configuration space
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
            let low = pci.read(bus.num, dev.dev, 0, i as u8 * 4);
            let high = pci.read(bus.num, dev.dev, 0, i as u8 * 4 + 2);
            let combined: u32 = (low as u32) | (high as u32) << 16;
            *v = combined;
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
        let bist_header: u16 = pci.read(bus.num, dev.dev, self.function, 14);
        let header: u8 = (bist_header & 0xFF) as u8;
        (header & 0x80) != 0
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
        map: &mut alloc::collections::btree_map::BTreeMap<u32, PciFunctionDriver>,
        pci: &mut PciConfigurationSpace,
    ) {
        for d in &self.devices {
            for f in &d.functions {
                let id = f.get_driver_id(pci, self, d);
                doors_macros2::kernel_print!("Checking pci device {:x}\r\n", id);
                if map.contains_key(&id) {
                    let code = map.get(&id).unwrap();
                    code.check(pci, self, d, f);
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
    pub fn driver_setup(&mut self) {
        let mut d = PCI_DRIVERS.lock();
        self.driver_run(&mut d);
    }
}

/// The trait for accessing pci configuration space
#[enum_dispatch::enum_dispatch]
trait PciConfigurationSpaceTrait {
    /// Read a configuration word
    fn read(&mut self, bus: u8, device: u8, function: u8, offset: u8) -> u16;
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
    /// Check to see if a pci device is present and able to be operated
    fn check(
        &self,
        cs: &mut PciConfigurationSpace,
        bus: &PciBus,
        dev: &PciDevice,
        function: &PciFunction,
    );

    /// Register the driver in the given map, must check to see if the driver is already registered
    fn register(&self, m: &mut BTreeMap<u32, PciFunctionDriver>);
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
    PciFunctionDriver::IntelPro1000(IntelPro1000 {}),
];

/// A dummy pci driver that does nothing
#[derive(Clone, Default)]
pub struct DummyPciFunctionDriver {}

impl PciFunctionDriverTrait for DummyPciFunctionDriver {
    fn check(
        &self,
        cs: &mut PciConfigurationSpace,
        bus: &PciBus,
        dev: &PciDevice,
        function: &PciFunction,
    ) {
    }

    fn register(&self, m: &mut BTreeMap<u32, PciFunctionDriver>) {
        if !m.contains_key(&0) {
            doors_macros2::kernel_print!("Register dummy pci driver\r\n");
            m.insert(0, self.clone().into());
        }
    }
}

/// Ethernet driver for the intel pro/1000 ethernet controller on pci
/// TODO: move this to crate::modules::network
#[derive(Clone, Default)]
pub struct IntelPro1000 {}

impl PciFunctionDriverTrait for IntelPro1000 {
    fn check(
        &self,
        cs: &mut PciConfigurationSpace,
        bus: &PciBus,
        dev: &PciDevice,
        function: &PciFunction,
    ) {
        doors_macros2::kernel_print!("Intel pro/1000 check function called\r\n");
    }

    fn register(&self, m: &mut BTreeMap<u32, PciFunctionDriver>) {
        if !m.contains_key(&0x8086100e) {
            doors_macros2::kernel_print!("Register intel pro/1000 pci driver\r\n");
            m.insert(0x8086100e, self.clone().into());
        }
    }
}

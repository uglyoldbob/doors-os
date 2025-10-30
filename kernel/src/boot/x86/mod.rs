//! The generic x86 module covering both 32 and 64-bit functionality.

use core::marker::PhantomData;

use crate::kernel::SystemTrait;
use crate::modules::interrupt::InterruptController;
use crate::modules::serial::SerialTrait;
use crate::IoReadWrite;
use crate::Locked;

#[cfg(target_arch = "x86_64")]
pub mod boot64;
use acpi::AcpiTables;
use alloc::boxed::Box;
#[cfg(target_arch = "x86_64")]
pub use boot64 as boot;

#[cfg(target_arch = "x86_64")]
use x86_64::instructions::port::{PortRead, PortWrite};

#[cfg(target_arch = "x86")]
pub mod boot32;
#[cfg(target_arch = "x86")]
pub use boot32 as boot;

pub use boot::mem2;

/// A generic interrupt or exception handler
type Handler = dyn FnMut() + Send + Sync;

/// The irq handlers registered by the system
static IRQ_HANDLERS: [Locked<Option<Box<Handler>>>; 256] = [const { Locked::new(None) }; 256];

/// The exception handlers registered by the system
static EXCEPTION_HANDLERS: [Locked<Option<Box<Handler>>>; 32] = [const { Locked::new(None) }; 32];

/// The entire list of io ports for an x86 machine
pub static IOPORTS: Locked<IoPortManager> = Locked::new(IoPortManager::new());

/// The heap for the kernel. This global allocator is responsible for the majority of dynamic memory in the kernel.
#[global_allocator]
static HEAP_MANAGER: Locked<mem2::HeapManager<'_, boot::memory::Page>> =
    Locked::new(mem2::HeapManager::new_mapping(
        &boot::VIRTUAL_MEMORY_ALLOCATOR,
        &crate::boot::VIRT_MEM_MAPPER,
    ));

/// A reference to a single io port
pub struct IoPortRef<T> {
    /// The address of the io port
    r: u16,
    /// Phantom data that the port contains
    _marker: PhantomData<T>,
}

impl IoReadWrite<u8> for IoPortRef<u8> {
    fn port_read(&mut self) -> u8 {
        unsafe {
            #[cfg(target_arch = "x86")]
            return x86::io::inb(self.r);
            #[cfg(target_arch = "x86_64")]
            return u8::read_from_port(self.r);
        }
    }

    fn port_write(&mut self, val: u8) {
        unsafe {
            #[cfg(target_arch = "x86")]
            x86::io::outb(self.r, val);
            #[cfg(target_arch = "x86_64")]
            u8::write_to_port(self.r, val);
        }
    }
}

impl IoReadWrite<u16> for IoPortRef<u16> {
    fn port_read(&mut self) -> u16 {
        unsafe {
            #[cfg(target_arch = "x86")]
            return x86::io::inw(self.r);
            #[cfg(target_arch = "x86_64")]
            return u16::read_from_port(self.r);
        }
    }

    fn port_write(&mut self, val: u16) {
        unsafe {
            #[cfg(target_arch = "x86")]
            x86::io::outw(self.r, val);
            #[cfg(target_arch = "x86_64")]
            u16::write_to_port(self.r, val);
        }
    }
}

impl IoReadWrite<u32> for IoPortRef<u32> {
    fn port_read(&mut self) -> u32 {
        unsafe {
            #[cfg(target_arch = "x86")]
            return x86::io::inl(self.r);
            #[cfg(target_arch = "x86_64")]
            return u32::read_from_port(self.r);
        }
    }

    fn port_write(&mut self, val: u32) {
        unsafe {
            #[cfg(target_arch = "x86")]
            x86::io::outl(self.r, val);
            #[cfg(target_arch = "x86_64")]
            u32::write_to_port(self.r, val);
        }
    }
}

/// An array of io ports.
pub struct IoPortArray<'a> {
    /// The first port address of the array.
    base: u16,
    /// The quantity of ports in the array.
    quantity: u16,
    /// A reference to the ioportmanager
    manager: &'a Locked<IoPortManager>,
}

impl Drop for IoPortArray<'_> {
    fn drop(&mut self) {
        self.manager.return_ports(self)
    }
}

impl IoPortArray<'_> {
    /// Get a port reference, ensuring that it is not out of bounds for the array. Will panic if out of bounds.
    pub fn port<T>(&self, index: u16) -> IoPortRef<T> {
        doors_macros::todo_item!("Figure out how to disallow port writes on the port ref for this");
        assert!(index < self.quantity);
        IoPortRef {
            r: self.base + index,
            _marker: PhantomData,
        }
    }

    /// Get the base address of the io address array
    pub fn get_base(&self) -> u16 {
        self.base
    }
}

/// Keeps track of all io ports on the system.
pub struct IoPortManager {
    /// A bitmap to track usage of all the ports for an x86 system
    ports: [usize; 65536 / core::mem::size_of::<usize>()],
}

impl Locked<IoPortManager> {
    /// Try to get a single port from the system
    pub fn get_port<T>(&self, base: u16) -> Option<IoPortRef<T>> {
        let mut manager = self.sync_lock();
        let p = base;
        let index = p / core::mem::size_of::<usize>() as u16;
        let shift = p % core::mem::size_of::<usize>() as u16;
        let d = manager.ports[index as usize] & (1 << shift);
        if d != 0 {
            None
        } else {
            manager.ports[index as usize] |= 1 << shift;
            Some(IoPortRef {
                r: base,
                _marker: PhantomData,
            })
        }
    }

    /// Try to get some io ports from the system.
    pub fn get_ports(&self, base: u16, quantity: u16) -> Option<IoPortArray<'_>> {
        let mut manager = self.sync_lock();
        let mut possible = true;
        for p in base..base + quantity {
            let index = p / core::mem::size_of::<usize>() as u16;
            let shift = p % core::mem::size_of::<usize>() as u16;
            let d = manager.ports[index as usize] & (1 << shift);
            if d != 0 {
                possible = false;
            }
        }
        if possible {
            for p in base..base + quantity {
                let index = p / core::mem::size_of::<usize>() as u16;
                let shift = p % core::mem::size_of::<usize>() as u16;
                manager.ports[index as usize] |= 1 << shift;
            }
            Some(IoPortArray {
                base,
                quantity,
                manager: self,
            })
        } else {
            None
        }
    }

    /// Returns a list of port previously obtained fromm the manager
    fn return_ports(&self, ports: &mut IoPortArray) {
        let mut manager = self.sync_lock();
        for p in ports.base..ports.base + ports.quantity {
            let index = p / core::mem::size_of::<usize>() as u16;
            let shift = p % core::mem::size_of::<usize>() as u16;
            manager.ports[index as usize] &= !(1 << shift);
        }
    }
}

impl Default for IoPortManager {
    fn default() -> Self {
        Self::new()
    }
}

impl IoPortManager {
    /// Create a new io port manager. All ports are assumed to be unused initially.
    pub const fn new() -> Self {
        Self {
            ports: [0; 65536 / core::mem::size_of::<usize>()],
        }
    }
}

extern "C" {
    /// Defines the start of the kernel for initial kernel load. This is defined by the linker script.
    pub static START_OF_KERNEL: u8;
    /// Defines the end of the kernel for the initial kernel load. This is defined by the linker script.
    pub static END_OF_KERNEL: u8;
    /// The pointer to the multiboot data
    static MULTIBOOT2_DATA: *const usize;
    /// The pointer to the end of the initial stack for the kernel
    static INITIAL_STACK: *const usize;
}

/// Setup timers for the x86 kernel
fn setup_timers() {
    let mut timers = crate::kernel::TIMERS.sync_lock();
    let pit = crate::modules::timer::x86::Pit::default();
    timers.register_timer(pit.into());
}

/// Probe and setup all serial ports for x86
/// This will probably be removed once pci space is further developed
fn setup_serial() {
    let mut serials = crate::kernel::SERIAL.sync_lock();
    for (base, irq) in [(0x3f8, 4), (0x2f8, 3), (0x3e8, 4), (0x2e8, 3)] {
        if let Some(com) = crate::modules::serial::x86::X86SerialPort::new(base, irq) {
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "Registered serial port {:x}\r\n",
                base
            ));
            let com = crate::modules::serial::Serial::PcComPort(com);
            use crate::modules::serial::SerialTrait;
            for i in 0..5 {
                com.sync_transmit_str(&alloc::format!("Testing the serial port {}\r\n", i));
            }
            serials.register_serial(com);
        }
    }
}

/// Enable interrupts for the first serial port if it is present
fn serial_interrupts() {
    let sys = crate::SYSTEM.read().clone();
    if let Some(s) = crate::kernel::SERIAL.take_device(0) {
        s.sync_transmit_str("About to enable async mode for serial port 0\r\n");
        s.enable_async(sys.clone()).unwrap();
        let t = s.convert(
            |a| a.make_text_display(),
            move |_t| {
                todo!();
            },
        );
        crate::common::VGA.sync_replace(Some(t));
    }
    if true {
        if let Some(s) = crate::kernel::SERIAL.take_device(1) {
            s.sync_transmit_str("About to enable async mode for serial port 1\r\n");
            s.enable_async(sys.clone()).unwrap();
        }
    }
}

#[derive(Clone)]
/// A structure for mapping and unmapping acpi memory
pub struct Acpi {
    /// The page manager for mapping and unmapping virtual memory
    pageman: &'static Locked<boot::memory::PagingTableManager<'static>>,
    /// The virtual memory manager for getting virtual memory
    vmm: &'static Locked<boot::memory::BumpAllocator>,
}

impl aml::Handler for boot::AmlHandler {
    fn sleep(&self, _milliseconds: u64) {
        todo!()
    }

    fn stall(&self, _microseconds: u64) {
        todo!()
    }

    fn read_u8(&self, _address: usize) -> u8 {
        crate::VGA.print_str("r1\r\n");
        todo!()
    }

    fn read_u16(&self, _address: usize) -> u16 {
        crate::VGA.print_str("r2\r\n");
        todo!()
    }

    fn read_u32(&self, _address: usize) -> u32 {
        crate::VGA.print_str("r3\r\n");
        todo!()
    }

    fn read_u64(&self, _address: usize) -> u64 {
        crate::VGA.print_str("r4\r\n");
        todo!()
    }

    fn write_u8(&mut self, _address: usize, _value: u8) {
        crate::VGA.print_str("w1\r\n");
        todo!()
    }

    fn write_u16(&mut self, _address: usize, _value: u16) {
        crate::VGA.print_str("w2\r\n");
        todo!()
    }

    fn write_u32(&mut self, _address: usize, _value: u32) {
        crate::VGA.print_str("w3\r\n");
        todo!()
    }

    fn write_u64(&mut self, _address: usize, _value: u64) {
        crate::VGA.print_str("w4\r\n");
        todo!()
    }

    fn read_io_u8(&self, _port: u16) -> u8 {
        crate::VGA.print_str("i1\r\n");
        todo!()
    }

    fn read_io_u16(&self, _port: u16) -> u16 {
        crate::VGA.print_str("i2\r\n");
        todo!()
    }

    fn read_io_u32(&self, _port: u16) -> u32 {
        crate::VGA.print_str("i3\r\n");
        todo!()
    }

    fn write_io_u8(&self, _port: u16, _value: u8) {
        crate::VGA.print_str("o1\r\n");
        todo!()
    }

    fn write_io_u16(&self, _port: u16, _value: u16) {
        crate::VGA.print_str("o2\r\n");
        todo!()
    }

    fn write_io_u32(&self, _port: u16, _value: u32) {
        crate::VGA.print_str("o3\r\n");
        todo!()
    }

    fn read_pci_u8(&self, _segment: u16, _bus: u8, _device: u8, _function: u8, _offset: u16) -> u8 {
        crate::VGA.print_str("pr1\r\n");
        todo!()
    }

    fn read_pci_u16(
        &self,
        _segment: u16,
        _bus: u8,
        _device: u8,
        _function: u8,
        _offset: u16,
    ) -> u16 {
        crate::VGA.print_str("pr2\r\n");
        todo!()
    }

    fn read_pci_u32(
        &self,
        _segment: u16,
        _bus: u8,
        _device: u8,
        _function: u8,
        _offset: u16,
    ) -> u32 {
        crate::VGA.print_str("pr3\r\n");
        todo!()
    }

    fn write_pci_u8(
        &self,
        _segment: u16,
        _bus: u8,
        _device: u8,
        _function: u8,
        _offset: u16,
        _value: u8,
    ) {
        crate::VGA.print_str("pw1\r\n");
        todo!()
    }

    fn write_pci_u16(
        &self,
        _segment: u16,
        _bus: u8,
        _device: u8,
        _function: u8,
        _offset: u16,
        _value: u16,
    ) {
        crate::VGA.print_str("pw2\r\n");
        todo!()
    }

    fn write_pci_u32(
        &self,
        _segment: u16,
        _bus: u8,
        _device: u8,
        _function: u8,
        _offset: u16,
        _value: u32,
    ) {
        crate::VGA.print_str("pw3\r\n");
        todo!()
    }
}

impl boot::X86System<'_> {
    /// This function loads the acpi tables if they are present
    #[doors_macros::config_check(acpi, "true")]
    fn load_acpi(&mut self) {
        crate::VGA.print_str("=== ACPI Initialization Starting ===\r\n");
        crate::VGA.print_str("Searching for RSDP...\r\n");
        let acpi = if let Some(rsdp2) = self.boot_info.rsdp_v2_tag() {
            crate::VGA.print_str("Found RSDPv2\r\n");
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "rsdpv2 at {:X} {:x} revision {}\r\n",
                rsdp2 as *const multiboot2::RsdpV2Tag as usize,
                rsdp2.xsdt_address(),
                rsdp2.revision()
            ));
            let result = unsafe {
                acpi::AcpiTables::from_rsdp(
                    self.acpi.as_ref().unwrap().handler().clone(),
                    rsdp2 as *const multiboot2::RsdpV2Tag as usize + 8,
                )
            };
            match &result {
                Ok(tables) => {
                    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                        "ACPI tables loaded successfully from RSDPv2, {} tables found\r\n",
                        tables.table_headers().count()
                    ));
                }
                Err(e) => {
                    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                        "Failed to load ACPI tables from RSDPv2: {:?}\r\n",
                        e
                    ));
                }
            }
            Some(result.unwrap())
        } else if let Some(rsdp1) = self.boot_info.rsdp_v1_tag() {
            crate::VGA.print_str("Found RSDPv1\r\n");
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "rsdpv1 at {:p} {:x}\r\n",
                rsdp1.signature().unwrap().as_ptr(),
                rsdp1.rsdt_address()
            ));
            let t = unsafe {
                acpi::AcpiTables::from_rsdp(
                    self.acpi.as_ref().unwrap().handler().clone(),
                    rsdp1.signature().unwrap().as_ptr() as usize,
                )
            };
            if let Err(e) = &t {
                crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                    "acpi error {:?}\r\n",
                    e
                ));
            }
            if let Ok(tables) = &t {
                crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                    "ACPI tables loaded successfully from RSDPv1, {} tables found\r\n",
                    tables.table_headers().count()
                ));
            }
            Some(t.unwrap())
        } else {
            crate::VGA.print_str("No RSDP found in boot info\r\n");
            None
        };
        if let Some(acpi) = acpi {
            crate::VGA.print_str("Adding ACPI tables to system\r\n");
            self.acpi = Some(self.acpi.take().unwrap().add_table(acpi));
        } else {
            crate::VGA.print_str("No ACPI tables found - RSDP not present\r\n");
        }
    }
}

impl boot::X86System<'_> {
    fn get_lapic(
        &self,
        acpi: &AcpiTables<Acpi>,
    ) -> Option<crate::modules::interrupt::x86::LocalApic> {
        for (address, header) in acpi.table_headers() {
            match header.signature {
                acpi::sdt::Signature::WAET => {}
                acpi::sdt::Signature::HPET => {}
                acpi::sdt::Signature::FADT => {}
                acpi::sdt::Signature::SSDT => {}
                acpi::sdt::Signature::MADT => match acpi.find_table::<acpi::sdt::madt::Madt>() {
                    None => {}
                    Some(madt) => {
                        let madt = madt.get();
                        for e in madt.entries() {
                            match e {
                                acpi::sdt::madt::MadtEntry::LocalApic(llapic) => {
                                    return Some(crate::modules::interrupt::x86::LocalApic::new());
                                }
                                _ => {}
                            }
                        }
                    }
                },
                _ => {}
            }
        }
        None
    }

    fn get_ioapic(
        &self,
        acpi: &AcpiTables<Acpi>,
    ) -> Option<crate::modules::interrupt::x86::IoApic> {
        for (address, header) in acpi.table_headers() {
            match header.signature {
                acpi::sdt::Signature::WAET => {}
                acpi::sdt::Signature::HPET => {}
                acpi::sdt::Signature::FADT => {}
                acpi::sdt::Signature::SSDT => {}
                acpi::sdt::Signature::MADT => match acpi.find_table::<acpi::sdt::madt::Madt>() {
                    None => {}
                    Some(madt) => {
                        let madt = madt.get();
                        for e in madt.entries() {
                            match e {
                                acpi::sdt::madt::MadtEntry::IoApic(ioapic) => {
                                    let paddr = ioapic.io_apic_address as usize;
                                    let vm = boot::VIRTUAL_MEMORY_ALLOCATOR
                                        .sync_lock()
                                        .allocate_nonram_memory(0x1000, 0x1000)
                                        .unwrap();
                                    let vaddr = crate::slice_address(unsafe { vm.as_ref() });
                                    boot::PAGING_MANAGER
                                        .sync_lock()
                                        .map_addresses_read_write(vaddr, paddr, 0x1000)
                                        .unwrap();
                                    let ioapic = crate::modules::interrupt::x86::IoApic::new(vaddr);
                                    return Some(ioapic);
                                }
                                _ => {}
                            }
                        }
                    }
                },
                _ => {}
            }
        }
        None
    }
}

impl crate::LockedArc<boot::X86System<'_>> {
    /// Perform processing necessary for acpi functionality
    #[doors_macros::config_check(acpi, "true")]
    fn handle_acpi(&self, aml: &mut aml::AmlContext) {
        crate::VGA.print_str("=== ACPI Handler Starting ===\r\n");
        let mut this = self.sync_lock();
        let acpi_system = this.acpi.as_ref();
        if acpi_system.is_none() {
            crate::VGA.print_str("ERROR: No ACPI system initialized\r\n");
            return;
        }

        let acpi_table = acpi_system.unwrap().table();
        if acpi_table.is_none() {
            crate::VGA.print_str("ERROR: No ACPI table available\r\n");
            return;
        }

        if let Some(acpi) = acpi_table {
            crate::VGA.print_str("ACPI tables available for processing\r\n");
            crate::VGA.print_str("Trying DSDT\r\n");
            if true {
                if let Ok(v) = acpi.dsdt() {
                    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                        "dsdt {:x} {:x}\r\n",
                        v.phys_address,
                        v.length
                    ));
                    boot::PAGING_MANAGER
                        .sync_lock()
                        .map_addresses_read_only(v.phys_address, v.phys_address, v.length as usize)
                        .unwrap();
                    let table: &[u8] = unsafe {
                        core::slice::from_raw_parts(v.phys_address as *const u8, v.length as usize)
                    };
                    if aml.parse_table(table).is_ok() {
                        crate::VGA.print_str("DSDT PARSED OK\r\n");
                    }
                }
            }
            if true {
                crate::VGA.print_str("About to iterate ssdts\r\n");
                let mut actual_ssdt_count = 0u32;
                for v in acpi.ssdts() {
                    actual_ssdt_count += 1;
                    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                        "ssdt {:x} {:x}\r\n",
                        v.phys_address,
                        v.length
                    ));

                    // Validate physical address and length
                    if v.phys_address == 0 {
                        crate::VGA.print_str("ERROR: SSDT has null physical address\r\n");
                        continue;
                    }
                    if v.length < 36 {
                        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                            "ERROR: SSDT length {} is too small (minimum 36 bytes)\r\n",
                            v.length
                        ));
                        continue;
                    }

                    // Attempt memory mapping
                    let map_result = boot::PAGING_MANAGER.sync_lock().map_addresses_read_only(
                        v.phys_address,
                        v.phys_address,
                        v.length as usize,
                    );

                    match map_result {
                        Ok(()) => {
                            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                                "Successfully mapped SSDT at {:x}, length {}\r\n",
                                v.phys_address,
                                v.length
                            ));
                        }
                        Err(e) => {
                            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                                "ERROR: Failed to map SSDT at {:x}: {:?}\r\n",
                                v.phys_address,
                                e
                            ));
                            continue;
                        }
                    }

                    let table: &[u8] = unsafe {
                        core::slice::from_raw_parts(v.phys_address as *const u8, v.length as usize)
                    };

                    // Verify table signature
                    if table.len() >= 4 {
                        let signature = &table[0..4];
                        let signature_str = core::str::from_utf8(signature).unwrap_or("????");
                        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                            "SSDT signature: '{}'\r\n",
                            signature_str
                        ));

                        if signature_str != "SSDT" {
                            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                                "WARNING: Expected 'SSDT' signature, got '{}'\r\n",
                                signature_str
                            ));
                        }
                    } else {
                        crate::VGA
                            .print_str("ERROR: Cannot read SSDT signature - table too short\r\n");
                    }
                    match aml.parse_table(table) {
                        Ok(()) => crate::VGA.print_str("SSDT PARSED OK\r\n"),
                        Err(e) => crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                            "SSDT PARSED ERR {:?}\r\n",
                            e
                        )),
                    }
                }
                crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                    "Actually processed {} SSDT tables via iterator\r\n",
                    actual_ssdt_count
                ));
            }

            let table_count = acpi.table_headers().count();
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "There are {} entries\r\n",
                table_count
            ));

            if table_count == 0 {
                crate::VGA.print_str("WARNING: No ACPI table headers found!\r\n");
            }

            let mut ssdt_found_in_headers = 0u32;
            let mut lapic = this.get_lapic(acpi);
            let mut ioapic = this.get_ioapic(acpi);

            for (address, header) in acpi.table_headers() {
                crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                    "sdt {:x} {:X} {} {} {}\r\n",
                    address,
                    &header as *const acpi::sdt::SdtHeader as usize,
                    header.signature.as_str(),
                    header.length as usize,
                    header.revision
                ));
                if header.signature.as_str() == "SSDT" {
                    ssdt_found_in_headers += 1;
                }
                match header.signature {
                    acpi::sdt::Signature::WAET => {
                        crate::VGA.print_str("TODO Parse the Waet table\r\n");
                    }
                    acpi::sdt::Signature::HPET => {}
                    acpi::sdt::Signature::FADT => {
                        match acpi.find_table::<acpi::sdt::fadt::Fadt>() {
                            Some(_fadt) => crate::VGA.print_str("TODO Parse the Fadt\r\n"),
                            None => crate::VGA.print_fixed_str(
                                doors_macros2::fixed_string_format!("FADT ERROR\r\n"),
                            ),
                        }
                    }
                    acpi::sdt::Signature::SSDT => {
                        let length = header.length; // Copy to avoid packed field reference
                        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                            "Found SSDT table at {:x}, length {}\r\n",
                            address,
                            length
                        ));
                    }
                    acpi::sdt::Signature::MADT => {
                        match acpi.find_table::<acpi::sdt::madt::Madt>() {
                            None => crate::VGA.print_fixed_str(
                                doors_macros2::fixed_string_format!("MADT ERROR \r\n",),
                            ),
                            Some(madt) => {
                                let madt = madt.get();
                                for e in madt.entries() {
                                    match e {
                                acpi::sdt::madt::MadtEntry::LocalApic(_llapic) => {
                                }
                                acpi::sdt::madt::MadtEntry::IoApic(_ioapic) => {
                                }
                                acpi::sdt::madt::MadtEntry::InterruptSourceOverride(i) => {
                                    crate::VGA.print_str(&alloc::format!("madt int source override {:?}\r\n", i));
                                    crate::VGA.sync_flush();
                                    if let Some(ioapic) = &mut ioapic {
                                        ioapic.register_override(i.irq, i.global_system_interrupt);
                                    }
                                }
                                acpi::sdt::madt::MadtEntry::NmiSource(_) => todo!(),
                                acpi::sdt::madt::MadtEntry::LocalApicNmi(_) => {
                                    crate::VGA.print_str("madt lapic nmi entry\r\n");
                                }
                                acpi::sdt::madt::MadtEntry::LocalApicAddressOverride(_) => todo!(),
                                acpi::sdt::madt::MadtEntry::IoSapic(_) => todo!(),
                                acpi::sdt::madt::MadtEntry::LocalSapic(_) => todo!(),
                                acpi::sdt::madt::MadtEntry::PlatformInterruptSource(_) => todo!(),
                                acpi::sdt::madt::MadtEntry::LocalX2Apic(_) => todo!(),
                                acpi::sdt::madt::MadtEntry::X2ApicNmi(_) => todo!(),
                                acpi::sdt::madt::MadtEntry::Gicc(_) => todo!(),
                                acpi::sdt::madt::MadtEntry::Gicd(_) => todo!(),
                                acpi::sdt::madt::MadtEntry::GicMsiFrame(_) => todo!(),
                                acpi::sdt::madt::MadtEntry::GicRedistributor(_) => todo!(),
                                acpi::sdt::madt::MadtEntry::GicInterruptTranslationService(_) => {
                                    todo!()
                                }
                                acpi::sdt::madt::MadtEntry::MultiprocessorWakeup(_) => todo!(),
                            }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            if let Some(mut lapic) = lapic {
                if let Some(ioapic) = ioapic {
                    crate::VGA.print_str("Registering ioapic\r\n");
                    lapic.register_ioapic(ioapic);
                }
                crate::VGA.print_str("Registering pic\r\n");
                crate::SYSTEM.read().disable_interrupts();
                let pic = crate::kernel::INTERRUPT_CONTROLLER.write().take();
                if let Some(crate::modules::interrupt::InterruptController::Pic(p)) = pic {
                    lapic.register_pic(p);
                }
                crate::kernel::INTERRUPT_CONTROLLER
                    .write()
                    .replace(lapic.into());
                crate::SYSTEM.read().enable_interrupts();
                crate::VGA.print_str("Registering pic done\r\n");
            }

            #[doors_macros::module_builtin_attr(hpet, "true")]
            if let Ok(info) = acpi::sdt::hpet::HpetInfo::new(acpi) {
                let vm = boot::VIRTUAL_MEMORY_ALLOCATOR
                    .sync_lock()
                    .allocate_nonram_memory(0x1000, 0x1000)
                    .unwrap();
                let vaddr = crate::slice_address(unsafe { vm.as_ref() });
                crate::VGA.print_str(&alloc::format!(
                    "Initializing HPET with {} channels at {:x}, {}, v{}\r\n",
                    info.num_comparators + 1,
                    info.base_address,
                    info.clock_tick_unit,
                    vaddr
                ));
                boot::PAGING_MANAGER
                    .sync_lock()
                    .map_addresses_read_write(vaddr, info.base_address, 0x1000)
                    .unwrap();
                {
                    let mut timers = crate::kernel::TIMERS.sync_lock();
                    for t in timers.iter_mut() {
                        let mut t2 = t.sync_lock();
                        {
                            let t3: &mut crate::modules::timer::Timer = &mut t2;
                            if let crate::modules::timer::Timer::X86Pit(pit) = t3 {
                                pit.disable();
                            }
                        }
                    }
                }
                let hpet = crate::modules::timer::hpet::Hpet::new(vaddr, info.num_comparators + 1);
                hpet.test();
                let mut timers = crate::kernel::TIMERS.sync_lock();
                timers.replace_pit(hpet.into());
            }

            {
                let l = crate::kernel::INTERRUPT_CONTROLLER.read();
                if let Some(InterruptController::Apic(l)) = l.as_ref() {
                    l.print();
                }
            }
        }
        if let Some(acpi) = this.acpi.take() {
            this.acpi = Some(acpi.to_platform());
        }

        // Initialize AML objects after all tables have been parsed
        crate::VGA.print_str("Initializing AML objects after table parsing...\r\n");
        match aml.initialize_objects() {
            Ok(()) => crate::VGA.print_str("AML objects initialized successfully\r\n"),
            Err(e) => crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "AML objects initialization failed: {:?}\r\n",
                e
            )),
        }
    }
}

/// Code that runs on startup that is common to both x86 and x86_64.
/// # Arguments
/// * start_kernel - The beginning of the kernel memory
/// * end_kernel - The end of used kernel memory
/// * boot_info - A reference to the boot information from the bootloader
/// * stack_end - The end of the kernel stack already setup
/// * stack_size - The amount of space reserved for the stack
/// * virtual_allocate_start - The first address that virtual memory should allocate to
/// * page_entries - The entries that are used to map page tables. 0 - the address for the usize entry, 1 - the virtual address controlled by the entry
fn start_common1(
    start_kernel: usize,
    end_kernel: usize,
    boot_info: &multiboot2::BootInformation,
    stack_end: usize,
    stack_size: usize,
    virtual_allocate_start: usize,
    page_entries: &[boot::memory::PageTableModifierData],
) {
    boot::VIRTUAL_MEMORY_ALLOCATOR
        .sync_lock()
        .relocate(start_kernel, end_kernel);
    boot::VIRTUAL_MEMORY_ALLOCATOR
        .sync_lock()
        .start_allocating(virtual_allocate_start);

    if let Some(mm) = boot_info.memory_map_tag() {
        let mut pal = boot::PAGE_ALLOCATOR.sync_lock();
        pal.init(mm);
        for area in mm
            .memory_areas()
            .iter()
            .filter(|i| i.typ() == multiboot2::MemoryAreaType::Available)
        {
            pal.add_memory_area(area);
        }
        pal.set_kernel_memory_used();
        let start_boot_info = crate::address(boot_info);
        let size_boot_info = boot_info.total_size();

        pal.set_area_used(start_boot_info, size_boot_info);
        pal.set_area_used(stack_end - stack_size, stack_size);
        pal.set_area_used(0, 0x100000);
        pal.done_adding_memory_areas();
    } else {
        panic!("Physical memory manager unavailable\r\n");
    };

    boot::VIRTUAL_MEMORY_ALLOCATOR
        .sync_lock()
        .stop_allocating(0x3fffff);

    for _ in 0..4 {
        let a = Box::<boot::memory::Page, &dyn core::alloc::Allocator>::new_uninit_in(
            &boot::VIRTUAL_MEMORY_ALLOCATOR,
        );
        let a = Box::leak(a);
    }

    boot::PAGING_MANAGER
        .sync_lock()
        .setup_from_existing(page_entries);

    HEAP_MANAGER.sync_lock().init_memory(10);

    if true {
        if true {
            let vga = crate::modules::video::vga::X86VgaMode::get(0xa0000).unwrap();
            let fb = crate::modules::video::Framebuffer::VgaHardware(vga);
            {
                let a = fb.make_console_palette(&crate::modules::video::MAIN_FONT_PALETTE);
                let mut v = crate::VGA.sync_lock();
                v.replace(crate::kernel::OwnedDevice::free_range(a));
            }
        } else {
            let vga = unsafe { crate::modules::video::text::X86VgaTextMode::get(0xb8000) };
            let b = crate::modules::video::TextDisplay::X86VgaTextMode(vga);
            let mut v = crate::VGA.sync_lock();
            v.replace(crate::kernel::OwnedDevice::free_range(b));
            drop(v);
        }
    }

    for _ in 0..4 {
        let a = Box::<boot::memory::Page, &dyn core::alloc::Allocator>::new_uninit_in(
            &boot::VIRTUAL_MEMORY_ALLOCATOR,
        );
        let a = Box::leak(a);
    }

    {
        let pic = crate::modules::interrupt::x86::Pic::new().unwrap();
        pic.disable();
        pic.remap(0x20, 0x28);
        crate::kernel::INTERRUPT_CONTROLLER
            .write()
            .replace(pic.into());
    }
}

/// This function is called by the entrance module for the kernel.
fn main_boot() -> ! {
    doors_macros::todo_item!("Use acpi tables to determine presence of ps2 hardware");
    let k = crate::modules::input::keyboard::Ps2::new();
    if let Some(k) = k {
        crate::common::KEYBOARD.write().replace(k);
    }
    crate::main()
}

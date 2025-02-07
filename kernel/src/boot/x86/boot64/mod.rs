//! This is the 64 bit module for x86 hardware. It contains the entry point for the 64-bit kernnel on x86.

use crate::modules::video::hex_dump_generic;
use crate::modules::video::TextDisplayTrait;
use crate::Locked;
use acpi::fadt::Fadt;
use acpi::hpet::HpetTable;
use acpi::madt::Madt;
use acpi::sdt::SdtHeader;
use acpi::AcpiHandler;
use acpi::PlatformInfo;
use alloc::boxed::Box;
use core::alloc::Allocator;
use core::ptr::NonNull;
use doors_macros::interrupt_64;
use doors_macros::interrupt_arg_64;
use lazy_static::lazy_static;
use raw_cpuid::{CpuId, CpuIdReaderNative};
use x86_64::structures::idt::InterruptStackFrame;

pub mod memory;

pub use memory::memory as mem2;

/// Driver for the APIC on x86 hardware
pub struct X86Apic {}

impl X86Apic {
    /// Retrieve an instance of the hardware
    pub fn get() -> Self {
        Self {}
    }
}

use x86_64::structures::{
    gdt::{Descriptor, GlobalDescriptorTable},
    idt::InterruptDescriptorTable,
};

#[no_mangle]
/// The global descriptor table for initial entry into long mode
pub static GDT_TABLE: GlobalDescriptorTable = make_gdt_table();

/// This function is responsible for building a gdt that can be built at compile time.
const fn make_gdt_table() -> GlobalDescriptorTable {
    let mut gdtb = GlobalDescriptorTable::new();
    gdtb.append(Descriptor::kernel_code_segment());
    gdtb.append(Descriptor::kernel_data_segment());
    gdtb
}

/// A struct for creating a global descriptor table pointer, suitable for loading with lidtr
#[repr(C, packed)]
pub struct GdtPointer<'a> {
    /// The size of the gdt table in bytes minus 1. See x86 processor manual for more information.
    size: u16,
    /// The address of the global descriptor table.
    address: &'a GlobalDescriptorTable,
}

#[repr(align(8))]
/// Holder structure for a Global descriptor table pointer, aligning the start of the structure as required.
pub struct GdtPointerHolder<'a> {
    /// The gdt pointer
    _d: GdtPointer<'a>,
}

/// The pointer used in assembly for entry into long mode, lidtr is used with this data structure.
#[no_mangle]
pub static GDT_TABLE_PTR: GdtPointerHolder = GdtPointerHolder {
    _d: GdtPointer {
        size: GDT_TABLE.limit(),
        address: &GDT_TABLE,
    },
};

extern "C" {
    static MULTIBOOT2_DATA: *const usize;
    static INITIAL_STACK: *const usize;
}

lazy_static! {
    static ref APIC: spin::Mutex<X86Apic> = spin::Mutex::new(X86Apic::get());
}

/// The divide by zero handler
#[interrupt_64]
pub extern "C" fn divide_by_zero() {
    doors_macros2::kernel_print!("Divide by zero\r\n");
    loop {
        x86_64::instructions::hlt();
    }
}

///The handler for segment not present
#[interrupt_arg_64]
pub extern "C" fn segment_not_present(arg: u32) {
    doors_macros2::kernel_print!("Segment not present {:x}\r\n", arg);
    loop {
        x86_64::instructions::hlt();
    }
}

/// The handler for the double fault exception
extern "x86-interrupt" fn double_fault_handler(
    sf: x86_64::structures::idt::InterruptStackFrame,
    error_code: u64,
) -> ! {
    doors_macros2::kernel_print!(
        "Double fault {:x} @ 0x{:X}\r\n",
        error_code,
        sf.instruction_pointer
    );
    loop {
        x86_64::instructions::hlt();
    }
}

/// Handles the page fault exception
extern "x86-interrupt" fn page_fault_handler(
    sf: x86_64::structures::idt::InterruptStackFrame,
    error_code: x86_64::structures::idt::PageFaultErrorCode,
) {
    doors_macros2::kernel_print!(
        "Page fault {:x} @ 0x{:X}\r\n",
        error_code,
        sf.instruction_pointer
    );
    doors_macros2::kernel_print!(
        "Fault address 0x{:X}\r\n",
        x86_64::registers::control::Cr2::read().unwrap()
    );
    loop {
        x86_64::instructions::hlt();
    }
}

/// Handles the invalid opcode exception
extern "x86-interrupt" fn invalid_opcode(sf: InterruptStackFrame) {
    doors_macros2::kernel_print!("Invalid opcode {:p}\r\n", &sf);
    doors_macros2::kernel_print!("Invalid opcode {:x}\r\n", sf.instruction_pointer.as_u64());
    loop {
        x86_64::instructions::hlt();
    }
}

/// A test interrupt handler
#[interrupt_arg_64]
pub extern "C" fn invalid_opcode2(sf: InterruptStackFrame) {
    doors_macros2::kernel_print!("Invalid opcode {:x}\r\n", sf.instruction_pointer.as_u64());
    loop {
        x86_64::instructions::hlt();
    }
}

/// A test interrupt handler
#[interrupt_64]
pub extern "C" fn unknown_interrupt() {
    doors_macros2::kernel_print!("Unknown interrupt fired\r\n");
    loop {
        x86_64::instructions::hlt();
    }
}

core::arch::global_asm!(include_str!("boot.s"));

/// The virtual memory allocator. Deleted space from this may not be reclaimable.
pub static VIRTUAL_MEMORY_ALLOCATOR: Locked<memory::BumpAllocator> =
    Locked::new(memory::BumpAllocator::new(0x1000));

/// The physical memory manager for the system
pub static PAGE_ALLOCATOR: Locked<memory::SimpleMemoryManager> =
    Locked::new(memory::SimpleMemoryManager::new(&VIRTUAL_MEMORY_ALLOCATOR));

/// The paging manager, which controls the memory management unit. Responsible for mapping virtual memory addresses to physical addresses.
pub static PAGING_MANAGER: Locked<memory::PagingTableManager> =
    Locked::new(memory::PagingTableManager::new(&PAGE_ALLOCATOR));

/// The interrupt descriptor table for the system
pub static INTERRUPT_DESCRIPTOR_TABLE: Locked<InterruptDescriptorTable> =
    Locked::new(InterruptDescriptorTable::new());

#[repr(align(16))]
#[derive(Copy, Clone)]
/// A structure for testing
struct Big {
    /// Some data to take up space
    _data: u128,
}

#[derive(Clone)]
/// A structure for mapping and unmapping acpi memory
struct Acpi<'a> {
    /// The page manager for mapping and unmapping virtual memory
    pageman: &'a Locked<memory::PagingTableManager<'a>>,
    /// The virtual memory manager for getting virtual memory
    vmm: &'a Locked<memory::BumpAllocator>,
}

impl acpi::AcpiHandler for &Acpi<'_> {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        if physical_address == 0 {
            log::error!("Received a null pointer request size {:x}", size);
            panic!("Received a null pointer request size");
        }
        if physical_address < (1 << 22) {
            acpi::PhysicalMapping::new(
                physical_address,
                NonNull::new(physical_address as *mut T).unwrap(),
                size,
                size,
                *self,
            )
        } else {
            doors_macros2::kernel_print!("ACPI MAP {:x} size {:x}\r\n", physical_address, size);
            let size_before_allocation = physical_address % core::mem::size_of::<memory::Page>();
            let end_remainder =
                (size_before_allocation + size) % core::mem::size_of::<memory::Page>();
            let size_after_allocation = if end_remainder > 0 {
                core::mem::size_of::<memory::Page>() - end_remainder
            } else {
                0
            };
            let start = physical_address - size_before_allocation;
            let realsize = size_before_allocation + size + size_after_allocation;

            let layout = core::alloc::Layout::from_size_align(
                realsize,
                core::mem::size_of::<memory::Page>(),
            )
            .unwrap();
            let buf = self.vmm.allocate(layout).unwrap();
            let bufaddr = crate::slice_address(buf.as_ref());
            doors_macros2::kernel_print!(
                "Got a virtual addres {:x}, size {:x}\r\n",
                bufaddr,
                buf.len()
            );

            let mut p = self.pageman.lock();
            let e = p.map_addresses_read_only(bufaddr, start, realsize);
            if e.is_err() {
                panic!("Unable to map acpi memory\r\n");
            }
            let vstart = bufaddr + size_before_allocation;

            let r = acpi::PhysicalMapping::new(
                physical_address,
                NonNull::new((vstart) as *mut T).unwrap(),
                realsize,
                size,
                *self,
            );
            doors_macros2::kernel_print!("Dumping mapped structure\r\n");
            hex_dump_generic(r.virtual_start().as_ref(), true, true);
            let _a: usize = r.virtual_start().addr().into();
            doors_macros2::kernel_print!(
                "ACPI PHYSICAL MAP virtual {:x} to physical {:x} size {:x} {:x}\r\n",
                r.virtual_start().as_ptr() as usize,
                r.physical_start(),
                r.region_length(),
                r.mapped_length()
            );
            doors_macros2::kernel_print!("sizeof T is {:x}\r\n", core::mem::size_of::<T>());
            r
        }
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        if region.physical_start() >= (1 << 22) {
            let acpi = acpi::PhysicalMapping::handler(region);
            let mut p = region.handler().pageman.lock();
            let s = region.virtual_start().as_ptr() as usize;
            let s = s - s % core::mem::size_of::<memory::Page>();
            let length = region.region_length();
            doors_macros2::kernel_print!(
                "ACPI UNMAP virtual {:x} physical {:x} size {:x} {:x}\r\n",
                region.virtual_start().as_ptr() as usize,
                region.physical_start(),
                length,
                region.mapped_length()
            );
            doors_macros2::kernel_print!("sizeof T is {:x}\r\n", core::mem::size_of::<T>());
            p.unmap_mapped_pages(s, length);
            let ptr = s as *mut u8;
            let layout =
                core::alloc::Layout::from_size_align(length, core::mem::size_of::<memory::Page>())
                    .unwrap();
            unsafe { acpi.vmm.deallocate(NonNull::new_unchecked(ptr), layout) };
        }
    }
}

/// Perform processing necessary for acpi functionality
fn handle_acpi(
    boot_info: &multiboot2::BootInformation,
    acpi_handler: impl AcpiHandler,
    aml: &mut aml::AmlContext,
) {
    doors_macros2::kernel_print!(
        "Size of acpi::fadt::Fadt is {:x}\r\n",
        core::mem::size_of::<acpi::fadt::Fadt>()
    );
    doors_macros2::kernel_print!(
        "Size of acpi::hpet::HpetTable is {:x}\r\n",
        core::mem::size_of::<acpi::hpet::HpetTable>()
    );
    doors_macros2::kernel_print!(
        "Size of acpi::madt::Madt is {:x}\r\n",
        core::mem::size_of::<acpi::madt::Madt>()
    );
    doors_macros2::kernel_print!(
        "Size of acpi::rsdp::Rsdp is {:x}\r\n",
        core::mem::size_of::<acpi::rsdp::Rsdp>()
    );

    let acpi = if let Some(rsdp2) = boot_info.rsdp_v2_tag() {
        doors_macros2::kernel_print!(
            "rsdpv2 at {:X} {:x} revision {}\r\n",
            rsdp2 as *const multiboot2::RsdpV2Tag as usize,
            rsdp2.xsdt_address(),
            rsdp2.revision()
        );
        Some(
            unsafe {
                acpi::AcpiTables::from_rsdp(
                    acpi_handler.clone(),
                    rsdp2 as *const multiboot2::RsdpV2Tag as usize + 8,
                )
            }
            .unwrap(),
        )
    } else if let Some(rsdp1) = boot_info.rsdp_v1_tag() {
        doors_macros2::kernel_print!(
            "rsdpv1 at {:p} {:x}\r\n",
            rsdp1.signature().unwrap().as_ptr(),
            rsdp1.rsdt_address()
        );
        let t = unsafe {
            acpi::AcpiTables::from_rsdp(
                acpi_handler.clone(),
                rsdp1.signature().unwrap().as_ptr() as usize,
            )
        };
        if let Err(e) = &t {
            doors_macros2::kernel_print!("acpi error {:?}\r\n", e);
        }
        if let Ok(t) = &t {
            doors_macros2::kernel_print!("ACPI ADDRESS {:p}\r\n", t);
        }
        Some(t.unwrap())
    } else {
        None
    };

    if acpi.is_none() {
        doors_macros2::kernel_print!("No ACPI table found\r\n");
    }
    let acpi = acpi.unwrap();
    doors_macros2::kernel_print!("acpi rev {:x}\r\n", acpi.revision());

    doors_macros2::kernel_print!("Trying DSDT\r\n");

    if true {
        if let Ok(v) = acpi.dsdt() {
            doors_macros2::kernel_print!("dsdt {:x} {:x}\r\n", v.address, v.length);
            PAGING_MANAGER
                .lock()
                .map_addresses_read_only(v.address, v.address, v.length as usize)
                .unwrap();
            let table: &[u8] =
                unsafe { core::slice::from_raw_parts(v.address as *const u8, v.length as usize) };
            if aml.parse_table(table).is_ok() {
                doors_macros2::kernel_print!("DSDT PARSED OK\r\n");
            }
        }
    }
    if true {
        doors_macros2::kernel_print!("About to iterate ssdts\r\n");
        for v in acpi.ssdts() {
            doors_macros2::kernel_print!("ssdt {:x} {:x}\r\n", v.address, v.length);
            PAGING_MANAGER
                .lock()
                .map_addresses_read_only(v.address, v.address, v.length as usize)
                .unwrap();
            let table: &[u8] =
                unsafe { core::slice::from_raw_parts(v.address as *const u8, v.length as usize) };
            match aml.parse_table(table) {
                Ok(()) => doors_macros2::kernel_print!("SSDT PARSED OK\r\n"),
                Err(e) => doors_macros2::kernel_print!("SSDT PARSED ERR {:?}\r\n", e),
            }
        }
    }

    doors_macros2::kernel_print!("There are {} entries\r\n", acpi.headers().count());

    for header in acpi.headers() {
        doors_macros2::kernel_print!(
            "sdt {:X} {} {} {}\r\n",
            &header as *const SdtHeader as usize,
            header.signature.as_str(),
            header.length as usize,
            header.revision
        );
        match header.signature {
            acpi::sdt::Signature::WAET => {
                doors_macros2::kernel_print!("TODO Parse the Waet table\r\n");
            }
            acpi::sdt::Signature::HPET => match acpi.find_table::<HpetTable>() {
                Ok(_hpet) => doors_macros2::kernel_print!("TODO Parse the Hpet table\r\n"),
                Err(e) => doors_macros2::kernel_print!("HPET ERROR {:?}\r\n", e),
            },
            acpi::sdt::Signature::FADT => match acpi.find_table::<Fadt>() {
                Ok(_fadt) => doors_macros2::kernel_print!("TODO Parse the Fadt\r\n"),
                Err(e) => doors_macros2::kernel_print!("FADT ERROR {:?}\r\n", e),
            },
            acpi::sdt::Signature::MADT => match acpi.find_table::<Madt>() {
                Err(e) => doors_macros2::kernel_print!("MADT ERROR {:?}\r\n", e),
                Ok(madt) => {
                    for e in madt.entries() {
                        match e {
                            acpi::madt::MadtEntry::LocalApic(lapic) => {
                                doors_macros2::kernel_print!(
                                    "madt lapic entry {:x} {:x} {:x}\r\n",
                                    lapic.processor_id,
                                    lapic.apic_id,
                                    { lapic.flags }
                                );
                            }
                            acpi::madt::MadtEntry::IoApic(_ioapic) => {
                                doors_macros2::kernel_print!("madt ioapic entry\r\n");
                            }
                            acpi::madt::MadtEntry::InterruptSourceOverride(_i) => {
                                doors_macros2::kernel_print!("madt int source override\r\n");
                            }
                            acpi::madt::MadtEntry::NmiSource(_) => todo!(),
                            acpi::madt::MadtEntry::LocalApicNmi(_) => {
                                doors_macros2::kernel_print!("madt lapic nmi entry\r\n");
                            }
                            acpi::madt::MadtEntry::LocalApicAddressOverride(_) => todo!(),
                            acpi::madt::MadtEntry::IoSapic(_) => todo!(),
                            acpi::madt::MadtEntry::LocalSapic(_) => todo!(),
                            acpi::madt::MadtEntry::PlatformInterruptSource(_) => todo!(),
                            acpi::madt::MadtEntry::LocalX2Apic(_) => todo!(),
                            acpi::madt::MadtEntry::X2ApicNmi(_) => todo!(),
                            acpi::madt::MadtEntry::Gicc(_) => todo!(),
                            acpi::madt::MadtEntry::Gicd(_) => todo!(),
                            acpi::madt::MadtEntry::GicMsiFrame(_) => todo!(),
                            acpi::madt::MadtEntry::GicRedistributor(_) => todo!(),
                            acpi::madt::MadtEntry::GicInterruptTranslationService(_) => todo!(),
                            acpi::madt::MadtEntry::MultiprocessorWakeup(_) => todo!(),
                        }
                    }
                }
            },
            _ => {}
        }
    }

    doors_macros2::kernel_print!("acpi: is {:p}\r\n", &acpi);

    let pi = PlatformInfo::new(&acpi);
    if let Ok(pi) = pi {
        doors_macros2::kernel_print!("pi: is {:p}\r\n", &pi);
    }
}

/// The programmable interrupt controller
struct Pic {
    /// The first pic
    pic1: super::IoPortArray<'static>,
    /// The second pic
    pic2: super::IoPortArray<'static>,
}

impl Pic {
    /// Get a pic object.
    pub fn new() -> Option<Self> {
        Some(Self {
            pic1: super::IOPORTS.get_ports(0x20, 2)?,
            pic2: super::IOPORTS.get_ports(0xa0, 2)?,
        })
    }

    /// Disable all interrupts for both pics
    pub fn disable(&mut self) {
        use crate::IoReadWrite;
        self.pic1.port(1).port_write(0xffu8);
        self.pic2.port(1).port_write(0xffu8);
    }

    /// Perform a remap of the pic interrupts
    /// # Arguments
    /// * offset1 - The amount to offset pic1 vectors by
    /// * offset2 - The amount to offset pic2 vectors by
    pub fn remap(&mut self, offset1: u8, offset2: u8) {
        use crate::IoReadWrite;
        let mut delay: super::IoPortRef<u8> = super::IOPORTS.get_port(0x80).unwrap();

        let mut pic1_cmd: super::IoPortRef<u8> = self.pic1.port(0);
        let mut pic1_data: super::IoPortRef<u8> = self.pic1.port(1);
        let mut pic2_cmd: super::IoPortRef<u8> = self.pic2.port(0);
        let mut pic2_data: super::IoPortRef<u8> = self.pic2.port(1);

        let mask1 = pic1_data.port_read();
        let mask2 = pic2_data.port_read();
        pic1_cmd.port_write(0x11);
        delay.port_write(0);
        pic2_cmd.port_write(0x11);
        delay.port_write(0);
        pic1_data.port_write(offset1);
        delay.port_write(0);
        pic2_data.port_write(offset2);
        delay.port_write(0);
        pic1_data.port_write(4);
        delay.port_write(0);
        pic2_data.port_write(2);
        delay.port_write(0);
        pic1_data.port_write(1);
        delay.port_write(0);
        pic2_data.port_write(1);
        delay.port_write(0);

        pic1_data.port_write(mask1);
        pic2_data.port_write(mask2);
    }
}

/// The registers for a local apic
#[repr(align(16))]
struct LocalApicRegister {
    /// The apic registers
    regs: [u32; 256],
}

/// Aml processing struct
struct AmlHandler {}

/// The system boot structure
#[doors_macros::config_check_struct]
pub struct X86System<'a> {
    #[doorsconfig = "acpi"]
    /// Used for information regarding the bootup of the kernel
    boot_info: multiboot2::BootInformation<'a>,
    #[doorsconfig = "acpi"]
    /// Used for acpi
    acpi_handler: Acpi<'a>,
    /// Used for cpuid stuff
    cpuid: CpuId<CpuIdReaderNative>,
    /// Phantom
    _phantom: core::marker::PhantomData<&'a usize>,
}

impl crate::kernel::SystemTrait for X86System<'_> {
    fn enable_interrupts(&self) {
        unsafe {
            INTERRUPT_DESCRIPTOR_TABLE.lock().load_unsafe();
            x86_64::instructions::interrupts::enable();
        }
    }

    fn idle(&mut self) {
        x86_64::instructions::hlt();
    }

    fn init(&mut self) {
        let aml_handler = Box::new(AmlHandler {});
        let mut aml = aml::AmlContext::new(aml_handler, aml::DebugVerbosity::All);
        if aml.initialize_objects().is_ok() {
            doors_macros2::kernel_print!("AML READY\r\n");
        }

        let cap = self.cpuid.get_processor_capacity_feature_info().unwrap();
        {
            let mut p = PAGING_MANAGER.lock();
            p.set_physical_address_size(cap.physical_address_bits());
            doors_macros2::kernel_print!("CPUID MAXADDR is {:?}\r\n", cap.physical_address_bits());
        }
        doors_macros::config_check_bool!(acpi, {
            doors_macros2::kernel_print!("About to open acpi stuff\r\n");
            handle_acpi(&self.boot_info, &self.acpi_handler, &mut aml);
            doors_macros2::kernel_print!("Done with acpi handling\r\n");
        });
        super::setup_pci();
    }
}

impl aml::Handler for AmlHandler {
    fn read_u8(&self, _address: usize) -> u8 {
        doors_macros2::kernel_print!("r1\r\n");
        todo!()
    }

    fn read_u16(&self, _address: usize) -> u16 {
        doors_macros2::kernel_print!("r2\r\n");
        todo!()
    }

    fn read_u32(&self, _address: usize) -> u32 {
        doors_macros2::kernel_print!("r3\r\n");
        todo!()
    }

    fn read_u64(&self, _address: usize) -> u64 {
        doors_macros2::kernel_print!("r4\r\n");
        todo!()
    }

    fn write_u8(&mut self, _address: usize, _value: u8) {
        doors_macros2::kernel_print!("w1\r\n");
        todo!()
    }

    fn write_u16(&mut self, _address: usize, _value: u16) {
        doors_macros2::kernel_print!("w2\r\n");
        todo!()
    }

    fn write_u32(&mut self, _address: usize, _value: u32) {
        doors_macros2::kernel_print!("w3\r\n");
        todo!()
    }

    fn write_u64(&mut self, _address: usize, _value: u64) {
        doors_macros2::kernel_print!("w4\r\n");
        todo!()
    }

    fn read_io_u8(&self, _port: u16) -> u8 {
        doors_macros2::kernel_print!("i1\r\n");
        todo!()
    }

    fn read_io_u16(&self, _port: u16) -> u16 {
        doors_macros2::kernel_print!("i2\r\n");
        todo!()
    }

    fn read_io_u32(&self, _port: u16) -> u32 {
        doors_macros2::kernel_print!("i3\r\n");
        todo!()
    }

    fn write_io_u8(&self, _port: u16, _value: u8) {
        doors_macros2::kernel_print!("o1\r\n");
        todo!()
    }

    fn write_io_u16(&self, _port: u16, _value: u16) {
        doors_macros2::kernel_print!("o2\r\n");
        todo!()
    }

    fn write_io_u32(&self, _port: u16, _value: u32) {
        doors_macros2::kernel_print!("o3\r\n");
        todo!()
    }

    fn read_pci_u8(&self, _segment: u16, _bus: u8, _device: u8, _function: u8, _offset: u16) -> u8 {
        doors_macros2::kernel_print!("pr1\r\n");
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
        doors_macros2::kernel_print!("pr2\r\n");
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
        doors_macros2::kernel_print!("pr3\r\n");
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
        doors_macros2::kernel_print!("pw1\r\n");
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
        doors_macros2::kernel_print!("pw2\r\n");
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
        doors_macros2::kernel_print!("pw3\r\n");
        todo!()
    }
}

/// The entry point for the 64 bit x86 kernel
#[no_mangle]
pub extern "C" fn start64() -> ! {
    let cpuid = raw_cpuid::CpuId::new();

    let boot_info = unsafe {
        multiboot2::BootInformation::load(
            MULTIBOOT2_DATA as *const multiboot2::BootInformationHeader,
        )
        .unwrap()
    };

    let start_kernel = unsafe { &super::START_OF_KERNEL } as *const u8 as usize;
    let end_kernel = unsafe { &super::END_OF_KERNEL } as *const u8 as usize;

    VIRTUAL_MEMORY_ALLOCATOR
        .lock()
        .relocate(start_kernel, end_kernel);
    VIRTUAL_MEMORY_ALLOCATOR.lock().start_allocating(unsafe {
        &memory::PAGE_DIRECTORY_BOOT1 as *const memory::PageTable as usize
    });

    if let Some(mm) = boot_info.memory_map_tag() {
        let mut pal = PAGE_ALLOCATOR.lock();
        pal.init(mm);
        for area in mm
            .memory_areas()
            .iter()
            .filter(|i| i.typ() == multiboot2::MemoryAreaType::Available)
        {
            pal.add_memory_area(area);
        }
        pal.set_kernel_memory_used();
        pal.done_adding_memory_areas();
    } else {
        panic!("Physical memory manager unavailable\r\n");
    };

    VIRTUAL_MEMORY_ALLOCATOR.lock().stop_allocating(0x3fffff);

    let apic: Box<LocalApicRegister, &Locked<memory::BumpAllocator>> =
        unsafe { Box::new_uninit_in(&VIRTUAL_MEMORY_ALLOCATOR).assume_init() };

    PAGING_MANAGER.lock().init();

    super::setup_serial();

    if false {
        if true {
            let vga = crate::modules::video::vga::X86VgaMode::get(0xa0000).unwrap();
            let fb = crate::modules::video::Framebuffer::VgaHardware(vga);
            {
                let a = fb.make_console_palette(&crate::modules::video::MAIN_FONT_PALETTE);
                let mut v = crate::VGA.lock();
                v.replace(a);
            }
        } else {
            let vga = unsafe { crate::modules::video::text::X86VgaTextMode::get(0xb8000) };
            let b = crate::modules::video::TextDisplay::X86VgaTextMode(vga);
            let mut v = crate::VGA.lock();
            v.replace(b);
            drop(v);
        }
    }

    let apic_msr = x86_64::registers::model_specific::Msr::new(0x1b);
    let apic_msr_value = unsafe { apic_msr.read() };
    let apic_address = apic_msr_value & 0xFFFFF000;

    PAGING_MANAGER
        .lock()
        .map_addresses_read_write(crate::address(apic.as_ref()), apic_address as usize, 0x400)
        .unwrap();
    doors_macros2::kernel_print!("APIC MSR IS {:x}\r\n", apic_msr_value);
    doors_macros2::kernel_print!("APIC RESERVED AT {:x?}\r\n", crate::address(apic.as_ref()));
    let apic_id = apic.regs[0x20 / 4];
    doors_macros2::kernel_print!("APIC ID IS {:x}\r\n", apic_id);
    let apic_version = apic.regs[0x30 / 4];
    doors_macros2::kernel_print!("APIC VERSION IS {:x}\r\n", apic_version);

    {
        let stack_end = unsafe { INITIAL_STACK as usize };
        let stack_size = 8 * 1024;
        PAGE_ALLOCATOR
            .lock()
            .set_area_used(stack_end - stack_size, stack_size);
    }

    if true {
        let test: alloc::boxed::Box<[u8; 4096], &Locked<memory::SimpleMemoryManager>> =
            alloc::boxed::Box::new_in([0; 4096], &PAGE_ALLOCATOR);
        doors_macros2::kernel_print!("test is {:x}\r\n", test.as_ref() as *const u8 as usize);
    }

    if true {
        let test: alloc::boxed::Box<[u8; 4096], &Locked<memory::SimpleMemoryManager>> =
            alloc::boxed::Box::new_in([0; 4096], &PAGE_ALLOCATOR);
        doors_macros2::kernel_print!("test2 is {:x}\r\n", test.as_ref() as *const u8 as usize);
    }

    let test: Box<[Big]> = Box::new([Big { _data: 5 }; 32]);
    doors_macros2::kernel_print!("test var is {:p}\r\n", test.as_ptr());
    drop(test);

    doors_macros2::kernel_print!("INTIAL STACK IS {:x}\r\n", unsafe {
        INITIAL_STACK as usize
    });

    let mut pic = Pic::new().unwrap();
    pic.disable();
    pic.remap(0x20, 0x28);

    {
        let mut idt = INTERRUPT_DESCRIPTOR_TABLE.lock();
        unsafe {
            idt[0].set_handler_addr(x86_64::addr::VirtAddr::from_ptr(
                divide_by_zero_asm as *const (),
            ));
            let mut entry = x86_64::structures::idt::Entry::missing();
            entry.set_handler_addr(x86_64::addr::VirtAddr::from_ptr(
                segment_not_present_asm as *const (),
            ));
            idt.segment_not_present = entry;

            let mut entry = x86_64::structures::idt::Entry::missing();
            entry.set_handler_addr(x86_64::addr::VirtAddr::from_ptr(
                double_fault_handler as *const (),
            ));
            idt.double_fault = entry;

            let mut entry = x86_64::structures::idt::Entry::missing();
            entry.set_handler_addr(x86_64::addr::VirtAddr::from_ptr(
                page_fault_handler as *const (),
            ));
            idt.page_fault = entry;

            let mut entry = x86_64::structures::idt::Entry::missing();
            entry.set_handler_addr(x86_64::addr::VirtAddr::from_ptr(
                invalid_opcode as *const (),
            ));
            idt.invalid_opcode = entry;
            //idt[6].set_handler_addr(x86_64::addr::VirtAddr::from_ptr(unknown_interrupt_asm as *const ()));
        }
    }

    let sys = doors_macros::config_build_struct! {
        X86System {
            #[doorsconfig = "acpi"]
            boot_info: boot_info,
            #[doorsconfig = "acpi"]
            acpi_handler: Acpi {
                pageman: &PAGING_MANAGER,
                vmm: &VIRTUAL_MEMORY_ALLOCATOR,
            },
            cpuid,
            _phantom: core::marker::PhantomData,
        }
    };

    super::main_boot(sys.into());
}

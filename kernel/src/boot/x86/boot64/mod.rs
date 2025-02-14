//! This is the 64 bit module for x86 hardware. It contains the entry point for the 64-bit kernnel on x86.

use crate::kernel;
use crate::modules::video::hex_dump_generic;
use crate::IoReadWrite;
use crate::Locked;
use crate::LockedArc;
use acpi::fadt::Fadt;
use acpi::hpet::HpetTable;
use acpi::madt::Madt;
use acpi::sdt::SdtHeader;
use acpi::AcpiHandler;
use acpi::PlatformInfo;
use alloc::boxed::Box;
use conquer_once::noblock::OnceCell;
use core::alloc::Allocator;
use core::pin::Pin;
use core::ptr::NonNull;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;
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

/// The irq handlers registered by the system
static IRQ_HANDLERS: OnceCell<LockedArc<[Option<Box<dyn FnMut() + Send + Sync>>; 256]>> =
    OnceCell::uninit();

/// The divide by zero handler
#[interrupt_64]
pub extern "C" fn divide_by_zero() {
    crate::VGA.print_str("Divide by zero\r\n");
    loop {
        x86_64::instructions::hlt();
    }
}

/// The irq4 handler
pub extern "x86-interrupt" fn irq4(_isf: InterruptStackFrame) {
    if let Ok(h) = IRQ_HANDLERS.try_get() {
        let mut h = h.sync_lock();
        if let Some(h2) = &mut h[4] {
            h2();
        }
    }
    let mut p = INTERRUPT_CONTROLLER.sync_lock();
    if let Some(p) = p.as_mut() {
        p.end_of_interrupt(4)
    }
    drop(p);
}

/// The irq7 handler
pub extern "x86-interrupt" fn irq7(_isf: InterruptStackFrame) {
    if let Ok(h) = IRQ_HANDLERS.try_get() {
        let mut h = h.sync_lock();
        if let Some(h2) = &mut h[7] {
            h2();
        }
    }
    let mut p = INTERRUPT_CONTROLLER.sync_lock();
    if let Some(p) = p.as_mut() {
        p.end_of_interrupt(7)
    }
    drop(p);
}

/// The irq11 handler
pub extern "x86-interrupt" fn irq11(_isf: InterruptStackFrame) {
    if let Ok(h) = IRQ_HANDLERS.try_get() {
        let mut h = h.sync_lock();
        if let Some(h2) = &mut h[11] {
            h2();
        } else {
            panic!();
        }
    }
    let mut p = INTERRUPT_CONTROLLER.sync_lock();
    if let Some(p) = p.as_mut() {
        p.end_of_interrupt(11)
    }
}

///The handler for segment not present
#[interrupt_arg_64]
pub extern "C" fn segment_not_present(arg: u32) {
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "Segment not present {:x}\r\n",
        arg
    ));
    loop {
        x86_64::instructions::hlt();
    }
}

/// The handler for the double fault exception
extern "x86-interrupt" fn double_fault_handler(
    sf: x86_64::structures::idt::InterruptStackFrame,
    error_code: u64,
) -> ! {
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "Double fault {:x} @ 0x{:X}\r\n",
        error_code,
        sf.instruction_pointer
    ));
    loop {
        x86_64::instructions::hlt();
    }
}

/// Handles the page fault exception
extern "x86-interrupt" fn page_fault_handler(
    sf: x86_64::structures::idt::InterruptStackFrame,
    error_code: x86_64::structures::idt::PageFaultErrorCode,
) {
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "Page fault {:x} @ 0x{:X}\r\n",
        error_code,
        sf.instruction_pointer
    ));
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "Fault address 0x{:X}\r\n",
        x86_64::registers::control::Cr2::read().unwrap()
    ));
    loop {
        x86_64::instructions::hlt();
    }
}

/// Handles the invalid opcode exception
extern "x86-interrupt" fn invalid_opcode(sf: InterruptStackFrame) {
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "Invalid opcode {:p}\r\n",
        &sf
    ));
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "Invalid opcode {:x}\r\n",
        sf.instruction_pointer.as_u64()
    ));
    loop {
        x86_64::instructions::hlt();
    }
}

/// A test interrupt handler
#[interrupt_arg_64]
pub extern "C" fn invalid_opcode2(sf: InterruptStackFrame) {
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "Invalid opcode {:x}\r\n",
        sf.instruction_pointer.as_u64()
    ));
    loop {
        x86_64::instructions::hlt();
    }
}

/// A test interrupt handler
#[interrupt_64]
pub extern "C" fn unknown_interrupt() {
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "Unknown interrupt fired\r\n"
    ));
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

/// The interrupt controller
static INTERRUPT_CONTROLLER: Locked<Option<Pic>> = Locked::new(None);

#[derive(Clone)]
/// A structure for mapping and unmapping acpi memory
struct Acpi<'a> {
    /// The page manager for mapping and unmapping virtual memory
    pageman: &'a Locked<memory::PagingTableManager<'a>>,
    /// The virtual memory manager for getting virtual memory
    vmm: &'a Locked<memory::BumpAllocator>,
}

impl acpi::AcpiHandler for Acpi<'_> {
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
                self.clone(),
            )
        } else {
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "ACPI MAP {:x} size {:x}\r\n",
                physical_address,
                size
            ));
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
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "Got a virtual addres {:x}, size {:x}\r\n",
                bufaddr,
                buf.len()
            ));

            let mut p = self.pageman.sync_lock();
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
                self.clone(),
            );
            crate::VGA.print_str("Dumping mapped structure\r\n");
            hex_dump_generic(r.virtual_start().as_ref(), true, true);
            let _a: usize = r.virtual_start().addr().into();
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "ACPI PHYSICAL MAP virtual {:x} to physical {:x} size {:x} {:x}\r\n",
                r.virtual_start().as_ptr() as usize,
                r.physical_start(),
                r.region_length(),
                r.mapped_length()
            ));
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "sizeof T is {:x}\r\n",
                core::mem::size_of::<T>()
            ));
            r
        }
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        if region.physical_start() >= (1 << 22) {
            let acpi = acpi::PhysicalMapping::handler(region);
            let mut p = region.handler().pageman.sync_lock();
            let s = region.virtual_start().as_ptr() as usize;
            let s = s - s % core::mem::size_of::<memory::Page>();
            let length = region.region_length();
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "ACPI UNMAP virtual {:x} physical {:x} size {:x} {:x}\r\n",
                region.virtual_start().as_ptr() as usize,
                region.physical_start(),
                length,
                region.mapped_length()
            ));
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "sizeof T is {:x}\r\n",
                core::mem::size_of::<T>()
            ));
            p.unmap_mapped_pages(s, length);
            let ptr = s as *mut u8;
            let layout =
                core::alloc::Layout::from_size_align(length, core::mem::size_of::<memory::Page>())
                    .unwrap();
            unsafe { acpi.vmm.deallocate(NonNull::new_unchecked(ptr), layout) };
        }
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

    /// Signal end of interrupt for the specified irq
    pub fn end_of_interrupt(&mut self, irq: u8) {
        if irq >= 8 {
            self.pic2.port(0).port_write(0x20u8);
        }
        self.pic1.port(0).port_write(0x20u8);
    }

    /// Disable all interrupts for both pics
    pub fn disable(&mut self) {
        use crate::IoReadWrite;
        self.pic1.port(1).port_write(0xffu8);
        self.pic2.port(1).port_write(0xffu8);
    }

    /// Enable the specified irq
    pub fn enable_irq(&mut self, irq: u8) {
        crate::VGA2.print_str("Enable an irq\r\n");
        if irq < 8 {
            let data: u8 = self.pic1.port(1).port_read();
            self.pic1.port(1).port_write(data & !(1 << irq));
        } else {
            let irq = irq - 8;
            let data: u8 = self.pic2.port(1).port_read();
            self.pic2.port(1).port_write(data & !(1 << irq));
        }
    }

    /// Disable the specified irq
    pub fn disable_irq(&mut self, irq: u8) {
        if irq < 8 {
            let data: u8 = self.pic1.port(1).port_read();
            self.pic1.port(1).port_write(data | (1 << irq));
        } else {
            let irq = irq - 8;
            let data: u8 = self.pic2.port(1).port_read();
            self.pic2.port(1).port_write(data | (1 << irq));
        }
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
    /// Suppress `Unpin` because this is self-referencing
    _pin: core::marker::PhantomPinned,
    /// Fake reference
    _phantom: core::marker::PhantomData<&'a usize>,
}

impl LockedArc<Pin<Box<X86System<'_>>>> {
    doors_macros::todo_item!("Create an attribute macro conditionally compile functions");
    /// Perform processing necessary for acpi functionality
    #[doors_macros::config_check(acpi, "true")]
    fn handle_acpi(&self, aml: &mut aml::AmlContext) {
        let this = self.sync_lock();
        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
            "Size of acpi::fadt::Fadt is {:x}\r\n",
            core::mem::size_of::<acpi::fadt::Fadt>()
        ));
        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
            "Size of acpi::hpet::HpetTable is {:x}\r\n",
            core::mem::size_of::<acpi::hpet::HpetTable>()
        ));
        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
            "Size of acpi::madt::Madt is {:x}\r\n",
            core::mem::size_of::<acpi::madt::Madt>()
        ));
        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
            "Size of acpi::rsdp::Rsdp is {:x}\r\n",
            core::mem::size_of::<acpi::rsdp::Rsdp>()
        ));

        let acpi = if let Some(rsdp2) = this.boot_info.rsdp_v2_tag() {
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "rsdpv2 at {:X} {:x} revision {}\r\n",
                rsdp2 as *const multiboot2::RsdpV2Tag as usize,
                rsdp2.xsdt_address(),
                rsdp2.revision()
            ));
            Some(
                unsafe {
                    acpi::AcpiTables::from_rsdp(
                        this.acpi_handler.clone(),
                        rsdp2 as *const multiboot2::RsdpV2Tag as usize + 8,
                    )
                }
                .unwrap(),
            )
        } else if let Some(rsdp1) = this.boot_info.rsdp_v1_tag() {
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "rsdpv1 at {:p} {:x}\r\n",
                rsdp1.signature().unwrap().as_ptr(),
                rsdp1.rsdt_address()
            ));

            let t = unsafe {
                acpi::AcpiTables::from_rsdp(
                    this.acpi_handler.clone(),
                    rsdp1.signature().unwrap().as_ptr() as usize,
                )
            };
            if let Err(e) = &t {
                crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                    "acpi error {:?}\r\n",
                    e
                ));
            }
            if let Ok(t) = &t {
                crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                    "ACPI ADDRESS {:p}\r\n",
                    t
                ));
            }
            Some(t.unwrap())
        } else {
            None
        };

        if acpi.is_none() {
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "No ACPI table found\r\n"
            ));
        }
        let acpi = acpi.unwrap();
        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
            "acpi rev {:x}\r\n",
            acpi.revision()
        ));

        crate::VGA.print_str("Trying DSDT\r\n");

        if true {
            if let Ok(v) = acpi.dsdt() {
                crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                    "dsdt {:x} {:x}\r\n",
                    v.address,
                    v.length
                ));
                PAGING_MANAGER
                    .sync_lock()
                    .map_addresses_read_only(v.address, v.address, v.length as usize)
                    .unwrap();
                let table: &[u8] = unsafe {
                    core::slice::from_raw_parts(v.address as *const u8, v.length as usize)
                };
                if aml.parse_table(table).is_ok() {
                    crate::VGA.print_str("DSDT PARSED OK\r\n");
                }
            }
        }
        if true {
            crate::VGA.print_str("About to iterate ssdts\r\n");
            for v in acpi.ssdts() {
                crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                    "ssdt {:x} {:x}\r\n",
                    v.address,
                    v.length
                ));
                PAGING_MANAGER
                    .sync_lock()
                    .map_addresses_read_only(v.address, v.address, v.length as usize)
                    .unwrap();
                let table: &[u8] = unsafe {
                    core::slice::from_raw_parts(v.address as *const u8, v.length as usize)
                };
                match aml.parse_table(table) {
                    Ok(()) => crate::VGA.print_str("SSDT PARSED OK\r\n"),
                    Err(e) => crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                        "SSDT PARSED ERR {:?}\r\n",
                        e
                    )),
                }
            }
        }

        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
            "There are {} entries\r\n",
            acpi.headers().count()
        ));

        for header in acpi.headers() {
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "sdt {:X} {} {} {}\r\n",
                &header as *const SdtHeader as usize,
                header.signature.as_str(),
                header.length as usize,
                header.revision
            ));
            match header.signature {
                acpi::sdt::Signature::WAET => {
                    crate::VGA.print_str("TODO Parse the Waet table\r\n");
                }
                acpi::sdt::Signature::HPET => match acpi.find_table::<HpetTable>() {
                    Ok(_hpet) => crate::VGA.print_str("TODO Parse the Hpet table\r\n"),
                    Err(e) => crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                        "HPET ERROR {:?}\r\n",
                        e
                    )),
                },
                acpi::sdt::Signature::FADT => match acpi.find_table::<Fadt>() {
                    Ok(_fadt) => crate::VGA.print_str("TODO Parse the Fadt\r\n"),
                    Err(e) => crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                        "FADT ERROR {:?}\r\n",
                        e
                    )),
                },
                acpi::sdt::Signature::MADT => match acpi.find_table::<Madt>() {
                    Err(e) => crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                        "MADT ERROR {:?}\r\n",
                        e
                    )),
                    Ok(madt) => {
                        for e in madt.entries() {
                            match e {
                                acpi::madt::MadtEntry::LocalApic(lapic) => {
                                    crate::VGA.print_fixed_str(
                                        doors_macros2::fixed_string_format!(
                                            "madt lapic entry {:x} {:x} {:x}\r\n",
                                            lapic.processor_id,
                                            lapic.apic_id,
                                            { lapic.flags }
                                        ),
                                    );
                                }
                                acpi::madt::MadtEntry::IoApic(_ioapic) => {
                                    crate::VGA.print_str("madt ioapic entry\r\n");
                                }
                                acpi::madt::MadtEntry::InterruptSourceOverride(_i) => {
                                    crate::VGA.print_str("madt int source override\r\n");
                                }
                                acpi::madt::MadtEntry::NmiSource(_) => todo!(),
                                acpi::madt::MadtEntry::LocalApicNmi(_) => {
                                    crate::VGA.print_str("madt lapic nmi entry\r\n");
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

        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
            "acpi: is {:p}\r\n",
            &acpi
        ));

        let pi = PlatformInfo::new(&acpi);
        if let Ok(pi) = pi {
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!("pi: is {:p}\r\n", &pi));
        }
    }
}

impl crate::kernel::SystemTrait for LockedArc<Pin<Box<X86System<'_>>>> {
    fn enable_interrupts(&self) {
        x86_64::instructions::interrupts::enable();
    }

    fn disable_interrupts(&self) {
        x86_64::instructions::interrupts::disable();
    }

    fn enable_irq(&self, irq: u8) {
        self.disable_interrupts();
        {
            let mut p = INTERRUPT_CONTROLLER.sync_lock();
            if let Some(p) = p.as_mut() {
                p.enable_irq(irq)
            }
        }
        self.enable_interrupts();
    }

    fn register_irq_handler<F: FnMut() + Send + Sync + 'static>(&self, irq: u8, handler: F) {
        let a = Box::new(handler);
        if let Ok(ih) = IRQ_HANDLERS.try_get() {
            let mut irqs = ih.sync_lock();
            irqs[irq as usize] = Some(a);
        }
    }

    fn disable_irq(&self, irq: u8) {
        let mut p = INTERRUPT_CONTROLLER.sync_lock();
        if let Some(p) = p.as_mut() {
            p.disable_irq(irq)
        }
    }

    fn idle(&self) {
        x86_64::instructions::hlt();
    }

    fn idle_if(&self, mut f: impl FnMut() -> bool) {
        self.disable_interrupts();
        if f() {
            crate::VGA2.print_str("IDLE NOW\r\n");
            x86_64::instructions::interrupts::enable_and_hlt();
        } else {
            self.enable_interrupts();
        }
    }

    async fn acpi_debug(&self) {
        crate::VGA.print_str_async("ACPI INFORMATION\r\n").await;
    }

    fn init(&self) {
        super::setup_serial();
        let aml_handler = Box::new(AmlHandler {});
        let mut aml = aml::AmlContext::new(aml_handler, aml::DebugVerbosity::All);
        aml.initialize_objects().unwrap();
        {
            let this = self.sync_lock();
            let cap = this.cpuid.get_processor_capacity_feature_info().unwrap();
            {
                let mut p = PAGING_MANAGER.sync_lock();
                p.set_physical_address_size(cap.physical_address_bits());
            }
        }

        doors_macros::config_check_bool!(acpi, {
            self.handle_acpi(&mut aml);
        });
        super::serial_interrupts();
    }
}

impl aml::Handler for AmlHandler {
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
        .sync_lock()
        .relocate(start_kernel, end_kernel);
    VIRTUAL_MEMORY_ALLOCATOR
        .sync_lock()
        .start_allocating(unsafe {
            &memory::PAGE_DIRECTORY_BOOT1 as *const memory::PageTable as usize
        });

    if let Some(mm) = boot_info.memory_map_tag() {
        let mut pal = PAGE_ALLOCATOR.sync_lock();
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

    VIRTUAL_MEMORY_ALLOCATOR
        .sync_lock()
        .stop_allocating(0x3fffff);

    let apic: Box<LocalApicRegister, &Locked<memory::BumpAllocator>> =
        unsafe { Box::new_uninit_in(&VIRTUAL_MEMORY_ALLOCATOR).assume_init() };

    PAGING_MANAGER.sync_lock().init();

    {
        let stack_end = unsafe { INITIAL_STACK as usize };
        let stack_size = 8 * 1024;
        PAGE_ALLOCATOR
            .sync_lock()
            .set_area_used(stack_end - stack_size, stack_size);
    }

    if true {
        if true {
            let vga = crate::modules::video::vga::X86VgaMode::get(0xa0000).unwrap();
            let fb = crate::modules::video::Framebuffer::VgaHardware(vga);
            {
                let a = fb.make_console_palette(&crate::modules::video::MAIN_FONT_PALETTE);
                let mut v = crate::VGA.sync_lock();
                v.replace(a);
            }
        } else {
            let vga = unsafe { crate::modules::video::text::X86VgaTextMode::get(0xb8000) };
            let b = crate::modules::video::TextDisplay::X86VgaTextMode(vga);
            let mut v = crate::VGA.sync_lock();
            v.replace(b);
            drop(v);
        }
    }

    let apic_msr = x86_64::registers::model_specific::Msr::new(0x1b);
    let apic_msr_value = unsafe { apic_msr.read() };
    let apic_address = apic_msr_value & 0xFFFFF000;

    PAGING_MANAGER
        .sync_lock()
        .map_addresses_read_write(crate::address(apic.as_ref()), apic_address as usize, 0x400)
        .unwrap();

    IRQ_HANDLERS
        .try_init_once(|| LockedArc::new([const { None }; 256]))
        .unwrap();

    {
        let mut pic = Pic::new().unwrap();
        pic.disable();
        pic.remap(0x20, 0x28);
        INTERRUPT_CONTROLLER.replace(Some(pic));
    }

    {
        let mut idt = INTERRUPT_DESCRIPTOR_TABLE.sync_lock();
        unsafe {
            idt[0].set_handler_addr(x86_64::addr::VirtAddr::from_ptr(
                divide_by_zero_asm as *const (),
            ));
            idt[0x24].set_handler_fn(irq4);
            idt[0x27].set_handler_fn(irq7);
            idt[0x2b].set_handler_fn(irq11);
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
        }
    }

    let sys = {
        let s = doors_macros::config_build_struct! {
            X86System {
                #[doorsconfig = "acpi"]
                boot_info: boot_info,
                #[doorsconfig = "acpi"]
                acpi_handler: Acpi {
                    pageman: &PAGING_MANAGER,
                    vmm: &VIRTUAL_MEMORY_ALLOCATOR,
                },
                cpuid,
                _pin: core::marker::PhantomPinned,
                _phantom: core::marker::PhantomData,
            }
        };
        let b = Box::new(s);
        doors_macros::todo_item!("Populated acpi stuff here");
        Box::into_pin(b)
    };

    unsafe {
        INTERRUPT_DESCRIPTOR_TABLE.sync_lock().load_unsafe();
    }

    crate::SYSTEM.replace(Some(kernel::System::X86_64(LockedArc::new(sys))));
    super::main_boot();
}

//! This module contains x86 32-bit specific code relating to how the machine boots up.

use crate::kernel;
use crate::LockedArc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr::NonNull;
use lazy_static::lazy_static;

mod gdt;
use gdt::GlobalDescriptorTable;
mod idt;
use idt::InterruptDescriptorTable;
pub mod memory;

pub use memory::memory as mem2;

use x86::segmentation::Descriptor;

/// Driver for the APIC on x86 hardware
pub struct X86Apic {}

impl X86Apic {
    /// Retrieve an instance of the hardware
    pub fn get() -> Self {
        Self {}
    }
}

/// The size of the main/boot kernel stack in bytes
pub const MAIN_STACK_SIZE: u32 = 8 * 1024;

/// This function is responsible for building a gdt that can be built at compile time.
const fn make_gdt_table() -> GlobalDescriptorTable {
    let mut gdtb = GlobalDescriptorTable::new();
    gdtb.const_add_entry(Descriptor {
        upper: 0b00000000110011111001101000000000,
        lower: 0xffff,
    });
    gdtb.const_add_entry(Descriptor {
        upper: 0b00000000110011111001001000000000,
        lower: 0xffff,
    });
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

/// The global descriptor table
pub static GDT_TABLE: GlobalDescriptorTable = make_gdt_table();

/// lidtr is used with this data structure.
#[no_mangle]
pub static GDT_TABLE_PTR: GdtPointerHolder = GdtPointerHolder {
    _d: GdtPointer {
        size: GDT_TABLE.limit(),
        address: &GDT_TABLE,
    },
};

lazy_static! {
    static ref APIC: spin::Mutex<X86Apic> = spin::Mutex::new(X86Apic::get());
}

/// The divide by zero handler
pub extern "x86-interrupt" fn divide_by_zero_exception() {
    crate::VGA.stop_async();
    crate::VGA.print_str("Divide by zero\r\n");
    loop {
        unsafe { x86::halt() };
    }
}

///The handler for segment not present
pub extern "x86-interrupt" fn segment_not_present_exception() {
    crate::VGA.stop_async();
    crate::VGA.print_str("Segment not present\r\n");
    loop {
        unsafe { x86::halt() };
    }
}

///Exception handler
pub extern "x86-interrupt" fn invalid_opcode_exception() {
    crate::VGA.stop_async();
    crate::VGA.print_str("Invalid opcode exception\r\n");
    loop {
        unsafe { x86::halt() };
    }
}

///Exception handler
pub extern "x86-interrupt" fn double_fault_exception() {
    crate::VGA.stop_async();
    crate::VGA.print_str("Double fault excpetion\r\n");
    loop {
        unsafe { x86::halt() };
    }
}
///Exception handler
pub extern "x86-interrupt" fn gpf_exception() {
    crate::VGA.stop_async();
    crate::VGA.print_str("Gpf exception\r\n");
    loop {
        unsafe { x86::halt() };
    }
}
///Exception handler
pub extern "x86-interrupt" fn page_fault_exception() {
    crate::VGA.stop_async();
    crate::VGA.print_str("Page fault exception\r\n");
    loop {
        unsafe { x86::halt() };
    }
}

/// The ending portion of an irq handler
pub fn finish_irq(irqnum: u8) {
    let p = super::INTERRUPT_CONTROLLER.read();
    if let Some(p) = p.as_ref() {
        p.end_of_interrupt(irqnum)
    }
}

/// The irq handler
pub extern "x86-interrupt" fn irq0() {
    let handle = super::IRQ_HANDLERS[0].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(0);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq handler
pub extern "x86-interrupt" fn irq3() {
    let handle = super::IRQ_HANDLERS[3].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(3);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq handler
pub extern "x86-interrupt" fn irq4() {
    let handle = super::IRQ_HANDLERS[4].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(4);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq handler
pub extern "x86-interrupt" fn irq7() {
    let handle = super::IRQ_HANDLERS[7].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(7);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq handler
pub extern "x86-interrupt" fn irq10() {
    let handle = super::IRQ_HANDLERS[10].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(10);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq handler
pub extern "x86-interrupt" fn irq15() {
    let handle = super::IRQ_HANDLERS[15].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(15);
    if let Some(h2) = h3 {
        h2();
    }
}

#[repr(align(16))]
#[derive(Copy, Clone)]
#[allow(unused)]
/// A structure for testing
struct Big {
    /// Some data to take up space
    data: u128,
}

/// Aml processing struct
struct AmlHandler {}

/// The system boot structure
#[doors_macros::config_check_struct]
#[allow(unused)]
pub struct X86System<'a> {
    /// Used for information regarding the bootup of the kernel
    boot_info: multiboot2::BootInformation<'a>,
    #[doorsconfig = "acpi"]
    /// Used for acpi
    acpi_handler: Acpi<'a>,
    /// The stack beginning
    stack_start: usize,
}

impl LockedArc<X86System<'_>> {
    /// Perform processing necessary for acpi functionality
    #[doors_macros::config_check(acpi, "true")]
    fn handle_acpi(&self) {
        let this = self.sync_lock();
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
                        let madt = madt.get();
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

impl crate::kernel::SystemTrait for LockedArc<X86System<'_>> {
    fn breakpoint(&self) -> Option<u8> {
        Some(0xcc)
    }

    fn register_exception_handler<F: FnMut() + Send + Sync + crate::Interrupt + 'static>(
        &self,
        exception: u8,
        handler: F,
    ) {
        let a = Box::new(handler);
        let mut irqs = super::EXCEPTION_HANDLERS[exception as usize].sync_lock();
        irqs.replace(a);
    }

    fn enable_interrupts(&self) {
        unsafe { x86::irq::enable() };
    }

    fn disable_interrupts(&self) {
        unsafe { x86::irq::disable() };
    }

    fn enable_irq(&self, irq: u8) {
        self.disable_interrupts_for(|| {
            let p = super::INTERRUPT_CONTROLLER.read();
            if let Some(p) = p.as_ref() {
                p.enable_irq(irq)
            }
        });
    }

    doors_macros::todo_item!("Add code for unregistering an irq handler");
    doors_macros::todo_item!("Return a Result here to detect shared irq attempts");
    fn register_irq_handler<F: FnMut() + Send + Sync + crate::Interrupt + 'static>(
        &self,
        irq: u8,
        handler: F,
    ) {
        let a = Box::new(handler);
        let mut irqs = super::IRQ_HANDLERS[irq as usize].sync_lock();
        irqs.replace(a);
    }

    fn disable_irq(&self, irq: u8) {
        self.disable_interrupts_for(|| {
            let p = super::INTERRUPT_CONTROLLER.read();
            if let Some(p) = p.as_ref() {
                p.disable_irq(irq)
            }
        });
    }

    async fn acpi_debug(&self) {
        crate::VGA.print_str_async("ACPI INFORMATION\r\n").await;
    }

    fn init(&self) {
        super::setup_timers();
        super::setup_serial();

        super::serial_interrupts();
        let _aml_handler = Box::new(AmlHandler {});
        /*
        let mut aml = aml::AmlContext::new(aml_handler, aml::DebugVerbosity::All);
        aml.initialize_objects().unwrap();
        */

        doors_macros::config_check_bool!(acpi, {
            self.handle_acpi();
        });
    }

    fn main_stack(&self) -> (usize, usize) {
        let s = self.sync_lock();
        (s.stack_start as usize, MAIN_STACK_SIZE as usize)
    }
}

#[derive(Clone)]
/// A structure for mapping and unmapping acpi memory
struct Acpi<'a> {
    /// The page manager for mapping and unmapping virtual memory
    pageman: &'a crate::Locked<memory::PagingTableManager<'a>>,
    /// The virtual memory manager for getting virtual memory
    vmm: &'a crate::Locked<memory::BumpAllocator>,
}

impl<'a> acpi::AcpiHandler for Acpi<'a> {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        if physical_address < (1 << 22) {
            acpi::PhysicalMapping::new(
                physical_address,
                NonNull::new(physical_address as *mut T).unwrap(),
                size,
                size,
                self.clone(),
            )
        } else {
            let start = physical_address - physical_address % core::mem::size_of::<memory::Page>();
            let presize = (physical_address + size) - start;
            let err = presize % core::mem::size_of::<memory::Page>();
            let realsize = if err != 0 {
                presize + (core::mem::size_of::<memory::Page>() - err)
            } else {
                presize
            };

            let mut b: Vec<u8, &crate::Locked<memory::BumpAllocator>> =
                Vec::with_capacity_in(realsize, self.vmm);
            let mut p = self.pageman.sync_lock();

            let e =
                p.map_addresses_read_only(b.as_ptr() as usize, start as usize, realsize as usize);
            if e.is_err() {
                panic!("Unable to map acpi memory\r\n");
            }
            let vstart = b.as_mut_ptr() as usize + err - size;

            let r = acpi::PhysicalMapping::new(
                start as usize,
                NonNull::new(vstart as *mut T).unwrap(),
                size,
                realsize,
                self.clone(),
            );
            b.leak();
            r
        }
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        if region.physical_start() >= (1 << 22) {
            let mut p = region.handler().pageman.sync_lock();
            let s = region.virtual_start().as_ptr() as usize;
            let s = s - s % core::mem::size_of::<memory::Page>() as usize;
            p.unmap_mapped_pages(s, region.mapped_length() as usize);
        }
    }
}

core::arch::global_asm!(include_str!("boot.s"));

/// The virtual memory allocator. Deleted space from this may not be reclaimable.
pub static VIRTUAL_MEMORY_ALLOCATOR: crate::Locked<memory::BumpAllocator> =
    crate::Locked::new(memory::BumpAllocator::new(0x1000));

/// The physical memory manager for the system
pub static PAGE_ALLOCATOR: crate::Locked<memory::SimpleMemoryManager> =
    crate::Locked::new(memory::SimpleMemoryManager::new(&VIRTUAL_MEMORY_ALLOCATOR));

/// The paging manager, which controls the memory management unit. Responsible for mapping virtual memory addresses to physical addresses.
pub static PAGING_MANAGER: crate::Locked<memory::PagingTableManager> =
    crate::Locked::new(memory::PagingTableManager::new(&PAGE_ALLOCATOR));

/// The interrupt descriptor table for the system
pub static INTERRUPT_DESCRIPTOR_TABLE: crate::Locked<InterruptDescriptorTable> =
    crate::Locked::new(InterruptDescriptorTable::new());

/// The entry point for the 32 bit x86 kernel
#[no_mangle]
pub extern "C" fn start32() -> ! {
    let _page =
        memory::Page4MbMapped::from_raw(unsafe { super::MULTIBOOT2_DATA as *const () as usize });

    let boot_info_addr = unsafe { super::MULTIBOOT2_DATA as usize };
    let boot_info = unsafe {
        multiboot2::BootInformation::load(
            boot_info_addr as *const multiboot2::BootInformationHeader,
        )
        .unwrap()
    };
    let start_kernel = unsafe { &super::START_OF_KERNEL } as *const u8 as usize;
    let end_kernel = unsafe { &super::END_OF_KERNEL } as *const u8 as usize;

    let stack_end = unsafe { super::INITIAL_STACK as usize };
    let stack_size = MAIN_STACK_SIZE as usize;

    super::start_common1(
        start_kernel,
        end_kernel,
        &boot_info,
        stack_end,
        stack_size,
        &memory::PAGE_DIRECTORY_BOOT1 as *const memory::PageTable as usize,
    );

    {
        let mut idt = INTERRUPT_DESCRIPTOR_TABLE.sync_lock();
        unsafe {
            idt.set_handler(0, divide_by_zero_exception);
            idt.set_handler(6, invalid_opcode_exception);
            idt.set_handler(8, double_fault_exception);
            idt.set_handler(11, segment_not_present_exception);
            idt.set_handler(13, gpf_exception);
            idt.set_handler(14, page_fault_exception);
            idt.set_handler(0x20, irq0);
            idt.set_handler(0x23, irq3);
            idt.set_handler(0x24, irq4);
            idt.set_handler(0x27, irq7);
            idt.set_handler(0x2a, irq10);
            idt.set_handler(0x2f, irq15);
        }
    }

    let sys = {
        doors_macros::config_build_struct! {
            X86System {
                boot_info: boot_info,
                #[doorsconfig = "acpi"]
                acpi_handler: Acpi {
                    pageman: &PAGING_MANAGER,
                    vmm: &VIRTUAL_MEMORY_ALLOCATOR,
                },
                stack_start: stack_end - stack_size,
            }
        }
    };

    unsafe {
        INTERRUPT_DESCRIPTOR_TABLE.sync_lock().load_unsafe();
    }

    *crate::SYSTEM.write() = kernel::System::X86_32(crate::LockedArc::new(sys));
    super::main_boot();
}

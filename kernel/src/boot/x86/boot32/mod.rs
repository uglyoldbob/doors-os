//! This module contains x86 32-bit specific code relating to how the machine boots up.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr::NonNull;
use doors_kernel_api::FixedString;
use doors_macros::interrupt;
use lazy_static::lazy_static;

mod gdt;
pub mod memory;

use super::VGA;

/// Driver for the APIC on x86 hardware
pub struct X86Apic {}

impl X86Apic {
    /// Retrieve an instance of the hardware
    pub fn get() -> Self {
        Self {}
    }
}

/// A generic message indicating the system is booting.
const GREETING: &str = "I am groot\r\n";

use x86::segmentation::BuildDescriptor;

/// Create a global descriptor table for the system to boot with.
fn make_gdt_table() -> gdt::GlobalDescriptorTable {
    let gdt = gdt::GlobalDescriptorTable::new();
    let code = x86::segmentation::DescriptorBuilder::code_descriptor(
        0,
        0xFFFFFFFF,
        x86::segmentation::CodeSegmentType::ExecuteRead,
    );
    let data = x86::segmentation::DescriptorBuilder::data_descriptor(
        0,
        0xFFFFFFFF,
        x86::segmentation::DataSegmentType::ReadWrite,
    );
    gdt.const_add_entry(code.finish())
        .const_add_entry(data.finish())
}

/// A struct for creating a global descriptor table pointer, suitable for loading with lidtr
#[repr(C, packed)]
pub struct GdtPointer<'a> {
    /// The size of the gdt
    size: u16,
    /// The address of the gdt
    address: &'a gdt::GlobalDescriptorTable,
}

#[repr(align(8))]
/// Holder structure for a Global descriptor table pointer, aligning the start of the structure as required.
pub struct GdtPointerHolder<'a> {
    /// The pointer for the gdt
    d: GdtPointer<'a>,
}

use doors_kernel_api::video::TextDisplay;
use x86::segmentation::SegmentDescriptorBuilder;

lazy_static! {
    static ref APIC: spin::Mutex<X86Apic> = spin::Mutex::new(unsafe { X86Apic::get() });
    static ref GDT_TABLE: gdt::GlobalDescriptorTable = make_gdt_table();
    static ref GDT_TABLE_PTR: GdtPointerHolder<'static> = GdtPointerHolder {
        d: GdtPointer {
            size: (GDT_TABLE.len() * 8 - 1) as u16,
            address: &GDT_TABLE,
        }
    };
}

/// The divide by zero handler
#[interrupt]
pub extern "C" fn divide_by_zero() {
    doors_macros2::kernel_print!("Divide by zero\r\n");
    loop {}
}

extern "C" {
    static MULTIBOOT2_DATA: *const usize;
}

///The handler for segment not present
#[interrupt]
pub extern "C" fn segment_not_present(arg: u32) {
    doors_macros2::kernel_print!("Segment not present {:x}\r\n", arg);
    loop {}
}

#[repr(align(16))]
#[derive(Copy, Clone)]
/// A structure for testing
struct Big {
    /// Some data to take up space
    data: u128,
}

/// The panic handler for the 32-bit kernel
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    doors_macros2::kernel_print!("PANIC AT THE DISCO!\r\n");
    if let Some(m) = info.payload().downcast_ref::<&str>() {
        doors_macros2::kernel_print!("{}", m);
    }

    if let Some(t) = info.location() {
        doors_macros2::kernel_print!("{}", t.file());
        doors_macros2::kernel_print!(" LINE {}\r\n", t.line());
    }
    doors_macros2::kernel_print!("PANIC SOMEWHERE ELSE!\r\n");
    loop {}
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
        doors_macros2::kernel_print!("acpi map {:x} {:x}\r\n", physical_address, size);
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
            let mut p = self.pageman.lock();

            let e =
                p.map_addresses_read_only(b.as_ptr() as usize, start as usize, realsize as usize);
            if e.is_err() {
                panic!("Unable to map acpi memory\r\n");
                loop {}
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
            let mut p = region.handler().pageman.lock();
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

/// A function to handle allocation errors in the kernel
#[alloc_error_handler]
pub fn whatever(l: core::alloc::Layout) -> ! {
    doors_macros2::kernel_print!("Failed to allocate\r\n");
    doors_macros2::kernel_print!("{:?}", l);
    loop {}
}

/// The entry point for the 32 bit x86 kernel
#[no_mangle]
pub extern "C" fn start32() -> ! {
    doors_macros2::kernel_print!("{}", GREETING);
    doors_macros2::kernel_print!("32 bit code\r\n");

    //Enable paging
    unsafe {
        memory::PAGE_DIRECTORY_BOOT1.entries[0] = 0x83;
        memory::PAGE_DIRECTORY_BOOT1.entries[1] = 0x200083;
        memory::PAGE_DIRECTORY_POINTER_TABLE.set_pagetable(0, &memory::PAGE_DIRECTORY_BOOT1);
        memory::PAGE_DIRECTORY_POINTER_TABLE.assign_to_cr3();
        let mut cr4 = x86::controlregs::cr4();
        cr4 |= x86::controlregs::Cr4::CR4_ENABLE_PAE | x86::controlregs::Cr4::CR4_ENABLE_PSE;
        x86::controlregs::cr4_write(cr4);
        let mut cr0 = x86::controlregs::cr0();
        cr0 |= x86::controlregs::Cr0::CR0_ENABLE_PAGING;
        x86::controlregs::cr0_write(cr0);
    }

    //let _cpuid = raw_cpuid::CpuId::new();

    let mbi = unsafe {
        multiboot2::BootInformation::load(
            MULTIBOOT2_DATA as *const multiboot2::BootInformationHeader,
        )
    };
    if let Err(e) = mbi {
        doors_macros2::kernel_print!("Failed mb load {:?}\r\n", e);
    }
    let boot_info = mbi.unwrap();

    let start_kernel = unsafe { &crate::START_OF_KERNEL } as *const u8 as usize;
    let end_kernel = unsafe { &crate::END_OF_KERNEL } as *const u8 as usize;

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
            doors_macros2::kernel_print!(
                "R {:x},S{:x} {:x} ,{:?}\r\n",
                area.start_address(),
                area.size(),
                area.end_address(),
                area.typ()
            );
            pal.add_memory_area(area);
        }
        pal.set_kernel_memory_used();
    } else {
        panic!("Physical memory manager unavailable\r\n");
    };
    VIRTUAL_MEMORY_ALLOCATOR.lock().stop_allocating(0x3fffff);
    PAGING_MANAGER.lock().init();

    if true {
        let test: alloc::boxed::Box<[u8; 4096], &crate::Locked<memory::SimpleMemoryManager>> =
            alloc::boxed::Box::new_in([0; 4096], &PAGE_ALLOCATOR);

        doors_macros2::kernel_print!("test is {:x}\r\n", test.as_ref() as *const u8 as usize);
    }

    if true {
        let test: alloc::boxed::Box<[u8; 4096], &crate::Locked<memory::SimpleMemoryManager>> =
            alloc::boxed::Box::new_in([0; 4096], &PAGE_ALLOCATOR);

        doors_macros2::kernel_print!("test2 is {:x}\r\n", test.as_ref() as *const u8 as usize);
    }

    let test: Box<[Big]> = Box::new([Big { data: 5 }; 32]);
    doors_macros2::kernel_print!("test var is {:p}\r\n", test.as_ptr());
    drop(test);

    let acpi_handler = Acpi {
        pageman: &PAGING_MANAGER,
        vmm: &VIRTUAL_MEMORY_ALLOCATOR,
    };

    let acpi = if let Some(rsdp2) = boot_info.rsdp_v2_tag() {
        doors_macros2::kernel_print!(
            "rsdpv2 at {:x} revision {}\r\n",
            rsdp2.xsdt_address() as *const u8 as usize,
            rsdp2.revision()
        );
        Some(
            unsafe {
                acpi::AcpiTables::from_rsdt(
                    acpi_handler,
                    rsdp2.revision(),
                    rsdp2.xsdt_address() as *const u8 as usize,
                )
            }
            .unwrap(),
        )
    } else if let Some(rsdp1) = boot_info.rsdp_v1_tag() {
        doors_macros2::kernel_print!(
            "rsdpv1 at {:x}\r\n",
            rsdp1.rsdt_address() as *const u8 as usize
        );
        let t = unsafe {
            acpi::AcpiTables::from_rsdt(acpi_handler, 0, rsdp1.rsdt_address() as *const u8 as usize)
        };
        if let Err(e) = &t {
            doors_macros2::kernel_print!("acpi error {:?}\r\n", e);
        }
        Some(t.unwrap())
    } else {
        None
    };

    if acpi.is_none() {
        doors_macros2::kernel_print!("No ACPI table found\r\n");
    }
    let acpi = acpi.unwrap();

    doors_macros2::kernel_print!("acpi rev {:x}\r\n", acpi.revision);

    for v in &acpi.ssdts {
        doors_macros2::kernel_print!("ssdt {:x} {:x}\r\n", v.address, v.length);
    }
    if let Some(v) = &acpi.dsdt {
        doors_macros2::kernel_print!("dsdt {:x} {:x}\r\n", v.address, v.length);
    }

    for (s, t) in &acpi.sdts {
        doors_macros2::kernel_print!(
            "sdt {} {:x} {:x} {}\r\n",
            s.as_str(),
            t.physical_address,
            t.length,
            t.validated
        );
    }

    let pi = acpi::PlatformInfo::new(&acpi);
    if let Ok(pi) = pi {
        doors_macros2::kernel_print!("pi: is {:p}\r\n", &pi);
    }

    unsafe {
        //x86_64::instructions::interrupts::enable();
    }
    super::main_boot();
}

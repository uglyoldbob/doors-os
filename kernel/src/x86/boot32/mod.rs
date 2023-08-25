use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr::NonNull;
use doors_kernel_api::FixedString;
use doors_macros::interrupt;
use lazy_static::lazy_static;

mod gdt;
mod memory;

use super::VGA;

/// Driver for the APIC on x86 hardware
pub struct X86Apic {}

impl X86Apic {
    /// Retrieve an instance of the hardware
    pub fn get() -> Self {
        Self {}
    }
}

const GREETING: &str = "I am groot\r\n";

use x86::segmentation::BuildDescriptor;

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
    size: u16,
    address: &'a gdt::GlobalDescriptorTable,
}

#[repr(align(8))]
/// Holder structure for a Global descriptor table pointer, aligning the start of the structure as required.
pub struct GdtPointerHolder<'a> {
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
    super::VGA.lock().print_str("Divide by zero\r\n");
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
    super::VGA.lock().print_str("PANIC AT THE DISCO!\r\n");
    if let Some(m) = info.payload().downcast_ref::<&str>() {
        super::VGA.lock().print_str(m);
    }

    if let Some(t) = info.location() {
        super::VGA.lock().print_str(t.file());
        doors_macros2::kernel_print!(" LINE {}\r\n", t.line());
    }
    super::VGA.lock().print_str("PANIC SOMEWHERE ELSE!\r\n");
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

            let e = p.map_addresses_read_only(b.as_ptr() as usize, start as usize, realsize as usize);
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
static VIRTUAL_MEMORY_ALLOCATOR: crate::Locked<memory::BumpAllocator> =
    crate::Locked::new(memory::BumpAllocator::new(0x1000));

/// The physical memory manager for the system
static PAGE_ALLOCATOR: crate::Locked<memory::SimpleMemoryManager> =
    crate::Locked::new(memory::SimpleMemoryManager::new(&VIRTUAL_MEMORY_ALLOCATOR));

/// The paging manager, which controls the memory management unit. Responsible for mapping virtual memory addresses to physical addresses.
static PAGING_MANAGER: crate::Locked<memory::PagingTableManager> =
    crate::Locked::new(memory::PagingTableManager::new(&PAGE_ALLOCATOR));

/// The heap for the kernel. This global allocator is responsible for the majority of dynamic memory in the kernel.
#[global_allocator]
static HEAP_MANAGER: crate::Locked<memory::HeapManager> = crate::Locked::new(
    memory::HeapManager::new(&PAGING_MANAGER, &VIRTUAL_MEMORY_ALLOCATOR),
);

/// The entry point for the 32 bit x86 kernel
#[no_mangle]
pub extern "C" fn start32() -> ! {
    super::VGA.lock().print_str(GREETING);
    super::VGA.lock().print_str("32 bit code\r\n");

    //Enable paging
    unsafe {
        memory::PAGE_DIRECTORY_BOOT1.entries[0] = 0x83;
        memory::PAGE_DIRECTORY_POINTER_TABLE.set_entry(0, &memory::PAGE_DIRECTORY_BOOT1);
        memory::PAGE_DIRECTORY_POINTER_TABLE.assign_to_cr3();
        let mut cr4 = x86::controlregs::cr4();
        cr4 |= x86::controlregs::Cr4::CR4_ENABLE_PAE | x86::controlregs::Cr4::CR4_ENABLE_PSE;
        x86::controlregs::cr4_write(cr4);
        let mut cr0 = x86::controlregs::cr0();
        cr0 |= x86::controlregs::Cr0::CR0_ENABLE_PAGING;
        x86::controlregs::cr0_write(cr0);
    }

    //let _cpuid = raw_cpuid::CpuId::new();

    let mbi = unsafe { multiboot2::BootInformation::load(MULTIBOOT2_DATA as *const multiboot2::BootInformationHeader) };
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
        for area in mm.memory_areas().iter().filter(|i| i.typ() == multiboot2::MemoryAreaType::Available) {
            doors_macros2::kernel_print!("R {:x},S{:x} {:x} ,{:?}\r\n",
                area.start_address(),
                area.size(),
                area.end_address(),
                area.typ());
            pal.add_memory_area(area);
        }
        pal.set_kernel_memory_used();
    } else {
        panic!("Physical memory manager unavailable\r\n");
    };

    VIRTUAL_MEMORY_ALLOCATOR.lock().stop_allocating();

    let b = Box::<memory::Page2Mb, &crate::Locked<memory::BumpAllocator>>::new_uninit_in(
        &VIRTUAL_MEMORY_ALLOCATOR,
    );
    let b =
        Box::<core::mem::MaybeUninit<memory::Page2Mb>, &crate::Locked<memory::BumpAllocator>>::leak(
            b,
        );
    let b = if (b.as_ptr() as u64) < (1 << 22) {
        let b = Box::<memory::Page2Mb, &crate::Locked<memory::BumpAllocator>>::new_uninit_in(
            &VIRTUAL_MEMORY_ALLOCATOR,
        );
        Box::<core::mem::MaybeUninit<memory::Page2Mb>, &crate::Locked<memory::BumpAllocator>>::leak(
            b,
        )
    } else {
        b
    };
    doors_macros2::kernel_print!("Got variable for init paging manager {:p}\r\n", b.as_ptr());
    PAGING_MANAGER.lock().init(b.as_ptr() as usize);

    if true {
        let test: alloc::boxed::Box<[u8; 4096], &crate::Locked<memory::SimpleMemoryManager>> =
            alloc::boxed::Box::new_in([0; 4096], &PAGE_ALLOCATOR);

        let mut tp: FixedString = FixedString::new();
        match core::fmt::write(
            &mut tp,
            format_args!("test is {:x}\r\n", test.as_ref() as *const u8 as usize),
        ) {
            Ok(_) => super::VGA.lock().print_str(tp.as_str()),
            Err(_) => super::VGA.lock().print_str("Error parsing string\r\n"),
        }
    }

    if true {
        let test: alloc::boxed::Box<[u8; 4096], &crate::Locked<memory::SimpleMemoryManager>> =
            alloc::boxed::Box::new_in([0; 4096], &PAGE_ALLOCATOR);

        let mut tp: FixedString = FixedString::new();
        match core::fmt::write(
            &mut tp,
            format_args!("test2 is {:x}\r\n", test.as_ref() as *const u8 as usize),
        ) {
            Ok(_) => super::VGA.lock().print_str(tp.as_str()),
            Err(_) => super::VGA.lock().print_str("Error parsing string\r\n"),
        }
    }

    let test: Box<[Big]> = Box::new([Big { data: 5 }; 32]);
    let mut tp: FixedString = FixedString::new();
    match core::fmt::write(&mut tp, format_args!("test var is {:p}\r\n", test.as_ptr())) {
        Ok(_) => super::VGA.lock().print_str(tp.as_str()),
        Err(_) => super::VGA.lock().print_str("Error parsing string\r\n"),
    }
    drop(test);

    let acpi_handler = Acpi {
        pageman: &PAGING_MANAGER,
        vmm: &VIRTUAL_MEMORY_ALLOCATOR,
    };

    let acpi = if let Some(rsdp2) = boot_info.rsdp_v2_tag() {
        let mut tp: FixedString = FixedString::new();
        match core::fmt::write(
            &mut tp,
            format_args!(
                "rsdpv2 at {:x} revision {}\r\n",
                rsdp2.xsdt_address() as *const u8 as usize,
                rsdp2.revision()
            ),
        ) {
            Ok(_) => super::VGA.lock().print_str(tp.as_str()),
            Err(_) => super::VGA.lock().print_str("Error parsing string\r\n"),
        }
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
        let mut tp: FixedString = FixedString::new();
        match core::fmt::write(
            &mut tp,
            format_args!(
                "rsdpv1 at {:x}\r\n",
                rsdp1.rsdt_address() as *const u8 as usize
            ),
        ) {
            Ok(_) => super::VGA.lock().print_str(tp.as_str()),
            Err(_) => super::VGA.lock().print_str("Error parsing string\r\n"),
        }
        let t = unsafe {
            acpi::AcpiTables::from_rsdt(acpi_handler, 0, rsdp1.rsdt_address() as *const u8 as usize)
        };
        if let Err(e) = &t {
            let mut tp: FixedString = FixedString::new();
            match core::fmt::write(&mut tp, format_args!("acpi error {:?}\r\n", e)) {
                Ok(_) => super::VGA.lock().print_str(tp.as_str()),
                Err(_) => super::VGA.lock().print_str("Error parsing string\r\n"),
            }
        }
        Some(t.unwrap())
    } else {
        None
    };

    if acpi.is_none() {
        super::VGA.lock().print_str("No ACPI table found\r\n");
    }
    let acpi = acpi.unwrap();

    let mut tp: FixedString = FixedString::new();
    match core::fmt::write(&mut tp, format_args!("acpi rev {:x}\r\n", acpi.revision)) {
        Ok(_) => super::VGA.lock().print_str(tp.as_str()),
        Err(_) => super::VGA.lock().print_str("Error parsing string\r\n"),
    }

    for v in acpi.ssdts {
        tp.clear();
        match core::fmt::write(
            &mut tp,
            format_args!("ssdt {:x} {:x}\r\n", v.address, v.length),
        ) {
            Ok(_) => super::VGA.lock().print_str(tp.as_str()),
            Err(_) => super::VGA.lock().print_str("Error parsing string\r\n"),
        }
    }
    if let Some(v) = acpi.dsdt {
        tp.clear();
        match core::fmt::write(
            &mut tp,
            format_args!("dsdt {:x} {:x}\r\n", v.address, v.length),
        ) {
            Ok(_) => super::VGA.lock().print_str(tp.as_str()),
            Err(_) => super::VGA.lock().print_str("Error parsing string\r\n"),
        }
    }

    for (s, t) in acpi.sdts {
        tp.clear();
        match core::fmt::write(
            &mut tp,
            format_args!(
                "sdt {} {:x} {:x} {}\r\n",
                s.as_str(),
                t.physical_address,
                t.length,
                t.validated
            ),
        ) {
            Ok(_) => super::VGA.lock().print_str(tp.as_str()),
            Err(_) => super::VGA.lock().print_str("Error parsing string\r\n"),
        }
    }

    unsafe {
        //x86_64::instructions::interrupts::enable();
    }
    super::main_boot();
}

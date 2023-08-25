//! This is the 64 bit module for x86 hardware. It contains the entry point for the 64-bit kernnel on x86.

use super::VGA;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ptr::NonNull;
use doors_kernel_api::FixedString;
use doors_macros::interrupt_64;
use doors_macros::interrupt_arg_64;
use lazy_static::lazy_static;

pub mod memory;

/// Driver for the APIC on x86 hardware
pub struct X86Apic {}

impl X86Apic {
    /// Retrieve an instance of the hardware
    pub fn get() -> Self {
        Self {}
    }
}

/// A greeting to prove that the kernel has started
const GREETING: &str = "I am groot\r\n";

use x86_64::structures::{
    gdt::{Descriptor, GlobalDescriptorTable},
    idt::InterruptDescriptorTable,
};
#[no_mangle]
/// The global descriptor table for initial entry into long mode
pub static GDT_TABLE: GlobalDescriptorTable = make_gdt_table();

/// This function is responsible for building a gdt that can be built at compile time.
const fn make_gdt_table() -> GlobalDescriptorTable {
    let (gdt, _segs) = GlobalDescriptorTable::from_descriptors([
        Descriptor::kernel_code_segment(),
        Descriptor::kernel_data_segment(),
    ]);
    gdt
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
    d: GdtPointer<'a>,
}

/// The pointer used in assembly for entry into long mode, lidtr is used with this data structure.
#[no_mangle]
pub static GDT_TABLE_PTR: GdtPointerHolder = GdtPointerHolder {
    d: GdtPointer {
        size: (GDT_TABLE.len() * 8 - 1) as u16,
        address: &GDT_TABLE,
    },
};

extern "C" {
    static MULTIBOOT2_DATA: *const usize;
}

use doors_kernel_api::video::TextDisplay;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = build_idt();
    static ref APIC: spin::Mutex<X86Apic> = spin::Mutex::new(X86Apic::get());
}

/// The divide by zero handler
#[interrupt_64]
pub extern "C" fn divide_by_zero() {
    super::VGA.lock().print_str("Divide by zero\r\n");
    loop {}
}

///The handler for segment not present
#[interrupt_arg_64]
pub extern "C" fn segment_not_present(arg: u32) {
    let mut a: FixedString = FixedString::new();
    core::fmt::write(&mut a, format_args!("Segment not present {:x}\r\n", arg))
        .expect("Error occurred while trying to write in String\r\n");
    super::VGA.lock().print_str(a.as_str());
    loop {}
}

/// The panic handler for the 64-bit kernel
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt;
    super::VGA.lock().print_str("PANIC AT THE DISCO!\r\n");
    if let Some(m) = info.payload().downcast_ref::<&str>() {
        super::VGA.lock().print_str(m);
    }

    if let Some(t) = info.location() {
        super::VGA.lock().print_str(t.file());
        let mut a: FixedString = FixedString::new();
        fmt::write(&mut a, format_args!("LINE {}\r\n", t.line()))
            .expect("Error occurred while trying to write in String");
        super::VGA.lock().print_str(a.as_str());
    }
    super::VGA.lock().print_str("PANIC SOMEWHERE ELSE!\r\n");
    loop {}
}

/// Used to build an interrupt descriptor table at runtime.
fn build_idt() -> InterruptDescriptorTable {
    let mut idt = InterruptDescriptorTable::new();
    unsafe {
        idt[0].set_handler_addr(x86_64::addr::VirtAddr::from_ptr(
            divide_by_zero_asm as *const (),
        ));
        let mut entry = x86_64::structures::idt::Entry::missing();
        entry.set_handler_addr(x86_64::addr::VirtAddr::from_ptr(
            segment_not_present_asm as *const (),
        ));
        idt.segment_not_present = entry;
    }
    idt
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

#[repr(align(16))]
#[derive(Copy, Clone)]
/// A structure for testing
struct Big {
    /// Some data to take up space
    data: u128,
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
        let mut tp: FixedString = FixedString::new();
        match core::fmt::write(
            &mut tp,
            format_args!("acpi map {:x} {:x}\r\n", physical_address, size),
        ) {
            Ok(_) => super::VGA.lock().print_str(tp.as_str()),
            Err(_) => super::VGA.lock().print_str("Error parsing string\r\n"),
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

/// The entry point for the 64 bit x86 kernel
#[no_mangle]
pub extern "C" fn start64() -> ! {
    super::VGA.lock().print_str(GREETING);
    let _cpuid = raw_cpuid::CpuId::new();

    let boot_info = unsafe {
        multiboot2::BootInformation::load(
            MULTIBOOT2_DATA as *const multiboot2::BootInformationHeader,
        )
        .unwrap()
    };

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
    PAGING_MANAGER.lock().init(b.as_ptr() as usize);

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
        super::VGA.lock().print_str("No ACPI table found\r\n");
    }
    let acpi = acpi.unwrap();
    doors_macros2::kernel_print!("acpi rev {:x}\r\n", acpi.revision);

    for v in acpi.ssdts {
        doors_macros2::kernel_print!("ssdt {:x} {:x}\r\n", v.address, v.length);
    }
    if let Some(v) = acpi.dsdt {
        doors_macros2::kernel_print!("dsdt {:x} {:x}\r\n", v.address, v.length);
    }

    for (s, t) in acpi.sdts {
        doors_macros2::kernel_print!(
            "sdt {} {:x} {:x} {}\r\n",
            s.as_str(),
            t.physical_address,
            t.length,
            t.validated
        );
    }

    unsafe {
        IDT.load_unsafe();
        //x86_64::instructions::interrupts::enable();
    }
    super::main_boot();
}

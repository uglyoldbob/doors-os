use doors_kernel_api::FixedString;
use doors_macros::interrupt;
use lazy_static::lazy_static;

mod gdt;
mod memory;

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

///The handler for segment not present
#[interrupt]
pub extern "C" fn segment_not_present(arg: u32) {
    let mut a: FixedString = FixedString::new();
    core::fmt::write(&mut a, format_args!("Segment not present {:x}\r\n", arg))
        .expect("Error occurred while trying to write in String\r\n");
    super::VGA.lock().print_str(a.as_str());
    loop {}
}

/// The panic handler for the 32-bit kernel
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

#[no_mangle]
pub fn start() {
    start32();
}

/// The entry point for the 32 bit x86 kernel
#[no_mangle]
pub extern "C" fn start32() -> ! {
    super::VGA.lock().print_str(GREETING);
    super::VGA.lock().print_str("32 bit code\r\n");

    unsafe {
        //x86_64::instructions::interrupts::enable();
    }
    super::main_boot();
}

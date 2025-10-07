//! This module contains x86 32-bit specific code relating to how the machine boots up.

use crate::kernel;
use crate::LockedArc;
use alloc::alloc::Allocator;
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

/// Defines the starting address for user space heap
pub const USER_SPACE_START: usize = 1 << 30;

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
pub extern "x86-interrupt" fn divide_by_zero_exception(_: u32) {
    crate::VGA.stop_async();
    crate::VGA.print_str("Divide by zero\r\n");
    loop {
        unsafe { x86::halt() };
    }
}

///The handler for segment not present
pub extern "x86-interrupt" fn segment_not_present_exception(_: u32, _code: u32) {
    crate::VGA.stop_async();
    crate::VGA.print_str("Segment not present\r\n");
    loop {
        unsafe { x86::halt() };
    }
}

///Exception handler
pub extern "x86-interrupt" fn invalid_opcode_exception(_: u32) {
    crate::VGA.stop_async();
    crate::VGA.print_str("Invalid opcode exception\r\n");
    loop {
        unsafe { x86::halt() };
    }
}

///Exception handler
pub extern "x86-interrupt" fn double_fault_exception(_: u32, _code: u32) {
    crate::VGA.stop_async();
    crate::VGA.print_str("Double fault excpetion\r\n");
    loop {
        unsafe { x86::halt() };
    }
}
///Exception handler
pub extern "x86-interrupt" fn gpf_exception(c1: u32, code: u32) {
    crate::VGA.stop_async();
    crate::VGA.print_str("Gpf exception\r\n");
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "{:x} {:x}\r\n",
        c1,
        code
    ));
    let table = (code >> 1) & 3;
    match table {
        0 => crate::VGA.print_str("GDT, "),
        2 => crate::VGA.print_str("LDT, "),
        _ => crate::VGA.print_str("IDT, "),
    }
    let index = (code >> 3) & 0x1FFF;
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!("0x{:x}\r\n", index));
    crate::VGA.sync_flush();
    loop {
        unsafe { x86::halt() };
    }
}
///Exception handler
pub extern "x86-interrupt" fn page_fault_exception(_: u32, _code: u32) {
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
pub extern "x86-interrupt" fn irq0(_: u32) {
    let handle = super::IRQ_HANDLERS[0].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(0);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq handler
pub extern "x86-interrupt" fn irq1(_: u32) {
    let handle = super::IRQ_HANDLERS[1].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(1);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq handler
pub extern "x86-interrupt" fn irq3(_: u32) {
    let handle = super::IRQ_HANDLERS[3].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(3);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq handler
pub extern "x86-interrupt" fn irq4(_: u32) {
    let handle = super::IRQ_HANDLERS[4].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(4);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq handler
pub extern "x86-interrupt" fn irq7(_: u32) {
    let handle = super::IRQ_HANDLERS[7].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(7);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq handler
pub extern "x86-interrupt" fn irq10(_: u32) {
    let handle = super::IRQ_HANDLERS[10].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(10);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq handler
pub extern "x86-interrupt" fn irq11(_: u32) {
    let handle = super::IRQ_HANDLERS[11].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(11);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq handler
pub extern "x86-interrupt" fn irq15(_: u32) {
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
pub struct AmlHandler {}

/// The holder for acpi stuff
pub enum AcpiStuff {
    /// Holds the acpi object
    Handler(super::Acpi),
    /// The acpi object with a table
    HandlerWithTable {
        /// The acpi object
        acpi: super::Acpi,
        /// The table set
        table: acpi::AcpiTables<super::Acpi>,
    },
    /// Holds the platform info object
    Platform(acpi::platform::AcpiPlatform<super::Acpi>),
}

impl AcpiStuff {
    /// Convert to a platform
    pub fn to_platform(self) -> Self {
        match self {
            Self::Handler(_acpi) => panic!(),
            Self::HandlerWithTable { acpi, table } => {
                let p = acpi::platform::AcpiPlatform::new(table, acpi).unwrap();
                crate::VGA
                    .print_fixed_str(doors_macros2::fixed_string_format!("pi: is {:p}\r\n", &p));
                Self::Platform(p)
            }
            Self::Platform(acpi_platform) => Self::Platform(acpi_platform),
        }
    }

    /// Get the platform
    pub fn platform(&self) -> Option<&acpi::platform::AcpiPlatform<super::Acpi>> {
        match self {
            Self::Handler(_acpi) => None,
            Self::HandlerWithTable { acpi: _, table: _ } => None,
            Self::Platform(acpi_platform) => Some(&acpi_platform),
        }
    }

    /// Get the optional table
    pub fn table(&self) -> Option<&acpi::AcpiTables<super::Acpi>> {
        match self {
            Self::Handler(_acpi) => None,
            Self::HandlerWithTable { acpi: _, table } => Some(table),
            Self::Platform(acpi_platform) => Some(&acpi_platform.tables),
        }
    }

    /// Adds the acpi table if necessary
    pub fn add_table(self, table: acpi::AcpiTables<super::Acpi>) -> Self {
        match self {
            Self::Handler(acpi) => Self::HandlerWithTable { acpi, table },
            Self::HandlerWithTable { acpi, table } => Self::HandlerWithTable { acpi, table },
            Self::Platform(_acpi_platform) => panic!(),
        }
    }

    /// Get the acpi handler
    pub fn handler(&self) -> &super::Acpi {
        match self {
            Self::Handler(acpi) => acpi,
            Self::HandlerWithTable { acpi, table: _ } => acpi,
            Self::Platform(acpi_platform) => &acpi_platform.handler,
        }
    }

    /// Get the acpi handler, mutably
    pub fn handler_mut(&mut self) -> &mut super::Acpi {
        match self {
            AcpiStuff::Handler(acpi) => acpi,
            Self::HandlerWithTable { acpi, table: _ } => acpi,
            AcpiStuff::Platform(acpi_platform) => &mut acpi_platform.handler,
        }
    }
}

/// The system boot structure
#[doors_macros::config_check_struct]
#[allow(unused)]
pub struct X86System<'a> {
    /// Used for information regarding the bootup of the kernel
    pub boot_info: multiboot2::BootInformation<'a>,
    #[doorsconfig = "acpi"]
    /// Used for acpi
    pub acpi: Option<AcpiStuff>,
    /// The stack beginning
    stack_start: usize,
}

impl crate::kernel::SystemTrait for LockedArc<X86System<'_>> {
    fn breakpoint(&self) -> Option<u8> {
        Some(0xcc)
    }

    fn create_process(&self, b: &object::File) -> Result<(), ()> {
        use object::Object;
        let text = b.section_by_name(".text");
        use object::ObjectSection;
        PAGE_ALLOCATOR.sync_lock().debug();
        if let Some(text) = text {
            crate::VGA.print_str(&alloc::format!(
                "About to run user process at {:x} {:x}\r\n",
                text.address(),
                USER_SPACE_START
            ));
            if text.address() == USER_SPACE_START as u64 {
                if let Ok(data) = text.data() {
                    PAGE_ALLOCATOR.debug();
                    let mut pt = PAGING_MANAGER.sync_lock().new_table();
                    PAGE_ALLOCATOR.debug();
                    crate::VGA.print_str(&alloc::format!(
                        "About to map pages with {} bytes for user process\r\n",
                        data.len()
                    ));
                    crate::VGA.print_str("Installing page table for user process\r\n");
                    unsafe {
                        pt.install();
                    }
                    for i in (0..data.len()).step_by(core::mem::size_of::<memory::Page>()) {
                        let user_address = i + USER_SPACE_START;
                        crate::VGA.print_str(&alloc::format!(
                            "About to map page {:x} at {:x}...",
                            i,
                            user_address
                        ));
                        pt.map_new_page(user_address)
                            .inspect(|_| crate::VGA.print_str("OK\r\n"))
                            .inspect_err(|_| {
                                crate::VGA.print_str("ERR\r\n");
                                loop {}
                            })?;
                        crate::VGA.print_str("Mapped a user page\r\n");
                    }
                    crate::VGA.print_str("About to copy data for user process\r\n");
                    let user_chunk = unsafe {
                        core::slice::from_raw_parts_mut(USER_SPACE_START as *mut u8, data.len())
                    };
                    let entry_addr = b.entry() as usize;
                    crate::VGA.print_str(&alloc::format!(
                        "About to run user program at {:x}\r\n",
                        entry_addr
                    ));
                    if (USER_SPACE_START..USER_SPACE_START + data.len()).contains(&entry_addr) {
                        user_chunk.copy_from_slice(data);
                        crate::VGA.print_str("About to spawn user thread\r\n");
                        let ptr = b.entry() as *const ();
                        let user_code: fn() = unsafe { core::mem::transmute(ptr) };
                        crate::scheduler::SCHEDULER
                            .read()
                            .as_ref()
                            .unwrap()
                            .spawn_thread(user_code);
                    } else {
                        crate::VGA.print_str(&alloc::format!(
                            "ERROR Start address {:x} not within {:x}..{:x}\r\n",
                            entry_addr,
                            USER_SPACE_START,
                            USER_SPACE_START + data.len()
                        ));
                    }
                }
            } else {
                crate::VGA.print_str(&alloc::format!(
                    "Text address is at {:x}\r\n",
                    text.address()
                ));
            }
        } else {
            crate::VGA.print_str("Text segment not found in object\r\n");
        }
        Ok(())
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

    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn acpi_debug(&self) {
        crate::VGA.print_str_async("ACPI INFORMATION\r\n").await;
    }

    fn init(&self) {
        super::setup_timers();
        super::setup_serial();

        super::serial_interrupts();
        let aml_handler = Box::new(AmlHandler {});
        let mut aml = aml::AmlContext::new(aml_handler, aml::DebugVerbosity::All);

        doors_macros::config_check_bool!(acpi, {
            self.handle_acpi(&mut aml);
        });
    }

    fn main_stack(&self) -> (usize, usize) {
        let s = self.sync_lock();
        (s.stack_start as usize, MAIN_STACK_SIZE as usize)
    }
}

impl acpi::Handler for super::Acpi {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        let size = if size < core::mem::size_of::<T>() {
            core::mem::size_of::<T>()
        } else {
            size
        };
        if physical_address == 0 {
            panic!("Received a null pointer request size");
        }
        if physical_address < (1 << 22) {
            acpi::PhysicalMapping {
                physical_start: physical_address,
                virtual_start: NonNull::new(physical_address as *mut T).unwrap(),
                region_length: size,
                mapped_length: size,
                handler: self.clone(),
            }
        } else {
            let size_before_allocation = physical_address % core::mem::size_of::<memory::Page>();
            let end_remainder =
                (size_before_allocation + size) % core::mem::size_of::<memory::Page>();
            let size_after_allocation = if end_remainder > 0 {
                core::mem::size_of::<memory::Page>() - end_remainder
            } else {
                0
            };
            let start = physical_address - size_before_allocation;
            let realsize = size_before_allocation + size + size_after_allocation + 0x1000;

            let layout = core::alloc::Layout::from_size_align(
                realsize,
                core::mem::size_of::<memory::Page>(),
            )
            .unwrap();
            let buf = self.vmm.allocate(layout).unwrap();
            let bufaddr = crate::slice_address(buf.as_ref());

            let mut p = self.pageman.sync_lock();
            let e = p.map_addresses_read_only(bufaddr, start, realsize);
            if e.is_err() {
                panic!("Unable to map acpi memory\r\n");
            }
            let vstart = bufaddr + size_before_allocation;

            acpi::PhysicalMapping {
                physical_start: physical_address,
                virtual_start: NonNull::new((vstart) as *mut T).unwrap(),
                region_length: size,
                mapped_length: size + size_after_allocation + 0x1000,
                handler: self.clone(),
            }
        }
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        if region.physical_start >= (1 << 22) {
            let mut p = region.handler.pageman.sync_lock();
            let s = region.virtual_start.as_ptr() as usize;
            let s = s - s % core::mem::size_of::<memory::Page>();
            let length = region.mapped_length;
            p.unmap_mapped_pages(s, length);
            let ptr = s as *mut u8;
            let layout =
                core::alloc::Layout::from_size_align(length, core::mem::size_of::<memory::Page>())
                    .unwrap();
            unsafe {
                region
                    .handler
                    .vmm
                    .deallocate(NonNull::new_unchecked(ptr), layout)
            };
        }
    }

    fn nanos_since_boot(&self) -> u64 {
        todo!()
    }

    fn create_mutex(&self) -> acpi::Handle {
        todo!()
    }

    fn acquire(&self, mutex: acpi::Handle, timeout: u16) -> Result<(), acpi::aml::AmlError> {
        todo!()
    }

    fn release(&self, mutex: acpi::Handle) {
        todo!()
    }

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

    fn write_u8(&self, _address: usize, _value: u8) {
        crate::VGA.print_str("w1\r\n");
        todo!()
    }

    fn write_u16(&self, _address: usize, _value: u16) {
        crate::VGA.print_str("w2\r\n");
        todo!()
    }

    fn write_u32(&self, _address: usize, _value: u32) {
        crate::VGA.print_str("w3\r\n");
        todo!()
    }

    fn write_u64(&self, _address: usize, _value: u64) {
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

    fn read_pci_u8(&self, address: acpi::PciAddress, offset: u16) -> u8 {
        crate::VGA.print_str("pr1\r\n");
        todo!()
    }

    fn read_pci_u16(&self, address: acpi::PciAddress, offset: u16) -> u16 {
        crate::VGA.print_str("pr1\r\n");
        todo!()
    }

    fn read_pci_u32(&self, address: acpi::PciAddress, offset: u16) -> u32 {
        crate::VGA.print_str("pr1\r\n");
        todo!()
    }

    fn write_pci_u8(&self, address: acpi::PciAddress, offset: u16, value: u8) {
        crate::VGA.print_str("pr1\r\n");
        todo!()
    }

    fn write_pci_u16(&self, address: acpi::PciAddress, offset: u16, value: u16) {
        crate::VGA.print_str("pr1\r\n");
        todo!()
    }

    fn write_pci_u32(&self, address: acpi::PciAddress, offset: u16, value: u32) {
        crate::VGA.print_str("pr1\r\n");
        todo!()
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

    let b = unsafe { &memory::TABLE3 } as *const u64 as usize;
    let c = unsafe { &memory::TABLE2 } as *const u64 as usize;
    let d = unsafe { &memory::TABLE1 } as *const u64 as usize;
    let page_entries = [
        memory::PageTableModifierData {
            virt: 0x400000,
            entry: b,
        },
        memory::PageTableModifierData {
            virt: 0x401000,
            entry: c,
        },
        memory::PageTableModifierData {
            virt: 0x402000,
            entry: d,
        },
    ];

    super::start_common1(
        start_kernel,
        end_kernel,
        &boot_info,
        stack_end,
        stack_size,
        &memory::PAGE_DIRECTORY_BOOT1 as *const memory::PageTable as usize,
        &page_entries,
    );

    {
        let mut idt = INTERRUPT_DESCRIPTOR_TABLE.sync_lock();
        unsafe {
            idt.set_handler_without_arg(0, divide_by_zero_exception);
            idt.set_handler_without_arg(6, invalid_opcode_exception);
            idt.set_handler(8, double_fault_exception);
            idt.set_handler(11, segment_not_present_exception);
            idt.set_handler(13, gpf_exception);
            idt.set_handler(14, page_fault_exception);
            idt.set_handler_without_arg(0x20, irq0);
            idt.set_handler_without_arg(0x21, irq1);
            idt.set_handler_without_arg(0x23, irq3);
            idt.set_handler_without_arg(0x24, irq4);
            idt.set_handler_without_arg(0x27, irq7);
            idt.set_handler_without_arg(0x2a, irq10);
            idt.set_handler_without_arg(0x2b, irq11);
            idt.set_handler_without_arg(0x2f, irq15);
        }
    }

    let mut sys = {
        doors_macros::config_build_struct! {
            X86System {
                boot_info: boot_info,
                #[doorsconfig = "acpi"]
                acpi: Some(AcpiStuff::Handler(super::Acpi {
                    pageman: &PAGING_MANAGER,
                    vmm: &VIRTUAL_MEMORY_ALLOCATOR,
                })),
                stack_start: stack_end - stack_size,
            }
        }
    };
    sys.load_acpi();
    unsafe {
        INTERRUPT_DESCRIPTOR_TABLE.sync_lock().load_unsafe();
    }

    *crate::SYSTEM.write() = kernel::System::X86_32(crate::LockedArc::new(sys));
    super::main_boot();
}

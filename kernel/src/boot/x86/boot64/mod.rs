//! This is the 64 bit module for x86 hardware. It contains the entry point for the 64-bit kernnel on x86.

use crate::boot::x86::memory::HeapManagerUserProcess;
use crate::boot::x86::memory::UserProcessAllocator;
use crate::kernel;
use crate::Locked;
use crate::LockedArc;
use alloc::boxed::Box;
use core::alloc::Allocator;
use core::ops::DerefMut;
use core::ptr::NonNull;
use lazy_static::lazy_static;
use raw_cpuid::{CpuId, CpuIdReaderNative};
use x86_64::structures::idt::InterruptStackFrame;

pub mod memory;

pub use memory::generic_memory as mem2;

/// Defines the starting address for user space heap
pub const USER_SPACE_START: usize = 1 << 39;

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

core::arch::global_asm!(include_str!("boot.s"));

/// The size of the main/boot kernel stack in bytes
pub const MAIN_STACK_SIZE: u64 = 8 * 1024;

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

lazy_static! {
    static ref APIC: spin::Mutex<X86Apic> = spin::Mutex::new(X86Apic::get());
}

/// The divide by zero handler
pub extern "x86-interrupt" fn divide_by_zero(_: usize) {
    crate::VGA.stop_async();
    crate::VGA.print_str("Divide by zero\r\n");
    loop {
        x86_64::instructions::hlt();
    }
}

doors_macros::todo_item!("Make a macro to build interrupt handlers on x86");

/// the debug exception handler
extern "x86-interrupt" fn debug_exception(_isf: InterruptStackFrame) {
    let mut handle = super::EXCEPTION_HANDLERS[1].sync_lock();
    if let Some(h) = handle.as_mut() {
        h();
    } else {
        loop {
            x86_64::instructions::hlt();
        }
    }
}

/// the breakpoint exception handler
extern "x86-interrupt" fn breakpoint_exception(_isf: InterruptStackFrame) {
    let mut handle = super::EXCEPTION_HANDLERS[3].sync_lock();
    if let Some(h) = handle.as_mut() {
        h();
    } else {
        loop {
            x86_64::instructions::hlt();
        }
    }
}

/// The ending portion of an irq handler
pub fn finish_irq(irqnum: u8) {
    let p = super::INTERRUPT_CONTROLLER.read();
    if let Some(p) = p.as_ref() {
        p.end_of_interrupt(irqnum)
    }
}

/// The irq0 handler
pub extern "x86-interrupt" fn irq0(_isf: InterruptStackFrame) {
    let handle = super::IRQ_HANDLERS[0].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(0);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq1 handler
pub extern "x86-interrupt" fn irq1(_isf: InterruptStackFrame) {
    let handle = super::IRQ_HANDLERS[1].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(1);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq3 handler
pub extern "x86-interrupt" fn irq3(_isf: InterruptStackFrame) {
    let mut handle = super::IRQ_HANDLERS[3].sync_lock();
    if let Some(h2) = handle.as_mut() {
        h2();
    }
    finish_irq(3);
}

/// The irq4 handler
pub extern "x86-interrupt" fn irq4(_isf: InterruptStackFrame) {
    let mut handle = super::IRQ_HANDLERS[4].sync_lock();
    if let Some(h2) = handle.as_mut() {
        h2();
    }
    finish_irq(4);
}

/// The irq7 handler
pub extern "x86-interrupt" fn irq7(_isf: InterruptStackFrame) {
    let mut handle = super::IRQ_HANDLERS[7].sync_lock();
    if let Some(h2) = handle.as_mut() {
        h2();
    }
    finish_irq(7);
}

/// The irq9 handler
pub extern "x86-interrupt" fn irq9(_isf: InterruptStackFrame) {
    let mut handle = super::IRQ_HANDLERS[9].sync_lock();
    if let Some(h2) = handle.as_mut() {
        h2();
    }
    finish_irq(9);
}

/// The irq10 handler
pub extern "x86-interrupt" fn irq10(_isf: InterruptStackFrame) {
    let mut handle = super::IRQ_HANDLERS[10].sync_lock();
    if let Some(h2) = handle.as_mut() {
        h2();
    }
    finish_irq(10);
}

/// The irq11 handler
pub extern "x86-interrupt" fn irq11(_isf: InterruptStackFrame) {
    let mut handle = super::IRQ_HANDLERS[11].sync_lock();
    if let Some(h2) = handle.as_mut() {
        h2();
    }
    finish_irq(11);
}

/// The irq12 handler
pub extern "x86-interrupt" fn irq12(_isf: InterruptStackFrame) {
    let handle = super::IRQ_HANDLERS[12].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(12);
    if let Some(h2) = h3 {
        h2();
    }
}

/// The irq15 handler
pub extern "x86-interrupt" fn irq15(_isf: InterruptStackFrame) {
    let mut handle = super::IRQ_HANDLERS[15].sync_lock();
    if let Some(h2) = handle.as_mut() {
        h2();
    }
    finish_irq(15);
}

/// The general protection fault handler
extern "x86-interrupt" fn general_protection_handler(isf: InterruptStackFrame, c: u64) {
    crate::VGA.stop_async();
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "General protection fault {:x} @{:x}\r\n",
        c,
        isf.instruction_pointer.as_u64()
    ));
    loop {
        x86_64::instructions::hlt();
    }
}

///The handler for segment not present
extern "x86-interrupt" fn segment_not_present(
    isf: x86_64::structures::idt::InterruptStackFrame,
    arg: u64,
) {
    crate::VGA.stop_async();
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "Segment not present@{:x} {:x}\r\n",
        isf.instruction_pointer.as_u64(),
        arg
    ));
    let table = (arg >> 1) & 3;
    match table {
        0 => crate::VGA.print_str("GDT, "),
        2 => crate::VGA.print_str("LDT, "),
        _ => crate::VGA.print_str("IDT, "),
    }
    let index = (arg >> 3) & 0x1FFF;
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!("0x{:x}\r\n", index));
    loop {
        x86_64::instructions::hlt();
    }
}

/// The handler for the double fault exception
extern "x86-interrupt" fn double_fault_handler(
    sf: x86_64::structures::idt::InterruptStackFrame,
    error_code: u64,
) {
    crate::VGA.stop_async();
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
    let a = x86_64::registers::control::Cr2::read().unwrap();
    crate::VGA.stop_async();
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "Page fault {:x} @ 0x{:X}, ",
        error_code,
        sf.instruction_pointer,
    ));
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!("{:X}\r\n", a.as_u64(),));
    crate::VGA.sync_flush();
    loop {
        x86_64::instructions::hlt();
    }
}

/// Handles the invalid opcode exception
extern "x86-interrupt" fn invalid_opcode(sf: InterruptStackFrame) {
    crate::VGA.stop_async();
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
pub extern "x86-interrupt" fn invalid_opcode2(sf: InterruptStackFrame) {
    crate::VGA.stop_async();
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "Invalid opcode {:x}\r\n",
        sf.instruction_pointer.as_u64()
    ));
    loop {
        x86_64::instructions::hlt();
    }
}

/// A test interrupt handler
pub extern "x86-interrupt" fn unknown_interrupt(_: usize) {
    crate::VGA.stop_async();
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "Unknown interrupt fired\r\n"
    ));
    loop {
        x86_64::instructions::hlt();
    }
}

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

impl acpi::AcpiHandler for super::Acpi {
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
            acpi::PhysicalMapping::new(
                physical_address,
                NonNull::new(physical_address as *mut T).unwrap(),
                size,
                size,
                self.clone(),
            )
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

            acpi::PhysicalMapping::new(
                physical_address,
                NonNull::new((vstart) as *mut T).unwrap(),
                size,
                size + size_after_allocation + 0x1000,
                self.clone(),
            )
        }
    }

    fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
        if region.physical_start() >= (1 << 22) {
            let acpi = acpi::PhysicalMapping::handler(region);
            let mut p = region.handler().pageman.sync_lock();
            let s = region.virtual_start().as_ptr() as usize;
            let s = s - s % core::mem::size_of::<memory::Page>();
            let length = region.mapped_length();
            p.unmap_mapped_pages(s, length);
            let ptr = s as *mut u8;
            let layout =
                core::alloc::Layout::from_size_align(length, core::mem::size_of::<memory::Page>())
                    .unwrap();
            unsafe { acpi.vmm.deallocate(NonNull::new_unchecked(ptr), layout) };
        }
    }
}

/// Aml processing struct
pub struct AmlHandler {}

/// The system boot structure
#[doors_macros::config_check_struct]
pub struct X86System<'a> {
    /// Used for information regarding the bootup of the kernel
    pub boot_info: multiboot2::BootInformation<'a>,
    #[doorsconfig = "acpi"]
    /// Used for acpi
    pub acpi_handler: super::Acpi,
    /// The acpi tables element
    pub acpi: Option<acpi::AcpiTables<super::Acpi>>,
    /// Used for cpuid stuff
    cpuid: CpuId<CpuIdReaderNative>,
    /// The stack beginning
    stack_start: usize,
}

impl crate::kernel::SystemTrait for LockedArc<X86System<'_>> {
    fn enable_interrupts(&self) {
        x86_64::instructions::interrupts::enable();
    }

    fn disable_interrupts(&self) {
        x86_64::instructions::interrupts::disable();
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
        {
            let this = self.sync_lock();
            let cap = this.cpuid.get_processor_capacity_feature_info().unwrap();
            {
                let mut p = PAGING_MANAGER.sync_lock();
                p.set_physical_address_size(cap.physical_address_bits());
            }
        }

        super::serial_interrupts();
        let aml_handler = Box::new(AmlHandler {});
        crate::VGA.print_str(&alloc::format!(
            "Size of aml is 0x{:x}\r\n",
            core::mem::size_of::<aml::AmlContext>()
        ));
        let mut aml = Box::new(aml::AmlContext::new(aml_handler, aml::DebugVerbosity::All));
        aml.initialize_objects().unwrap();

        doors_macros::config_check_bool!(acpi, {
            self.handle_acpi(&mut aml);
        });
    }

    fn main_stack(&self) -> (usize, usize) {
        let s = self.sync_lock();
        (s.stack_start as usize, MAIN_STACK_SIZE as usize)
    }

    fn create_process(&self, b: &object::File) -> Result<(), ()> {
        use object::Object;
        let text = b.section_by_name(".text");
        use object::ObjectSection;
        PAGE_ALLOCATOR.sync_lock().debug();
        if let Some(text) = text {
            if text.address() == USER_SPACE_START as u64 {
                if let Ok(data) = text.data() {
                    PAGE_ALLOCATOR.debug();
                    let mut pt = PAGING_MANAGER.sync_lock().new_table();
                    x86_64::instructions::bochs_breakpoint();
                    let heap = UserProcessAllocator::new(HeapManagerUserProcess::new(
                        &PAGING_MANAGER,
                        USER_SPACE_START,
                    ));
                    PAGE_ALLOCATOR.debug();
                    crate::VGA.print_str(&alloc::format!(
                        "About to map pages with {} bytes for user process\r\n",
                        data.len()
                    ));
                    for i in (0..data.len()).step_by(core::mem::size_of::<memory::Page>()) {
                        let user_address = i + USER_SPACE_START;
                        crate::VGA.print_str(&alloc::format!(
                            "About to map page {:x} at {:x}...",
                            i,
                            user_address
                        ));
                        pt.map_new_page(user_address)
                            .inspect(|_| crate::VGA.print_str("OK\r\n"))
                            .inspect_err(|_| crate::VGA.print_str("ERR\r\n"))?;
                        crate::VGA.print_str("Mapped a user page\r\n");
                    }
                    crate::VGA.print_str("Installing page table for user process\r\n");
                    loop {}
                    unsafe {
                        pt.install();
                    }
                    crate::VGA.print_str("About to copy data for user process\r\n");
                    pt.setup_cache(USER_SPACE_START);
                    let user_chunk = unsafe {
                        core::slice::from_raw_parts_mut(USER_SPACE_START as *mut u8, data.len())
                    };
                    user_chunk.copy_from_slice(data);
                    crate::VGA.print_str("About to spawn user thread\r\n");
                    crate::scheduler::SCHEDULER
                        .read()
                        .as_ref()
                        .unwrap()
                        .spawn_thread(|| {
                            crate::VGA.print_str("A user process stub function is running\r\n")
                        });
                    crate::VGA.print_str("GOT USER BINARY TEXT data\r\n");
                }
            }
        }
        Ok(())
    }
}

/// The entry point for the 64 bit x86 kernel
#[no_mangle]
pub extern "C" fn start64() -> ! {
    let cpuid = raw_cpuid::CpuId::new();

    let start_kernel = unsafe { &super::START_OF_KERNEL } as *const u8 as usize;
    let end_kernel = unsafe { &super::END_OF_KERNEL } as *const u8 as usize;

    //Copy the boot information header to the end of the kernel, update the end of the kernel variable to reflect the new data
    let bi_size = {
        let boot_info = unsafe {
            multiboot2::BootInformation::load(
                super::MULTIBOOT2_DATA as *const multiboot2::BootInformationHeader,
            )
            .unwrap()
        };
        let size = boot_info.total_size();
        let dest = unsafe { core::slice::from_raw_parts_mut(end_kernel as *mut u8, size) };
        let source =
            unsafe { core::slice::from_raw_parts_mut(boot_info.start_address() as *mut u8, size) };
        if crate::slice_address(dest) < crate::slice_address(source) {
            let di = dest.iter_mut();
            let si = source.iter();
            let a = si.zip(di);
            for (s, d) in a {
                *d = *s;
            }
        } else {
            let di = dest.iter_mut();
            let si = source.iter();
            let a = si.zip(di);
            for (s, d) in a.rev() {
                *d = *s;
            }
        }
        size
    };

    let boot_info = unsafe {
        multiboot2::BootInformation::load(end_kernel as *const multiboot2::BootInformationHeader)
            .unwrap()
    };
    let end_kernel = end_kernel + bi_size;

    let stack_end = unsafe { super::INITIAL_STACK as usize };
    let stack_size = MAIN_STACK_SIZE as usize;

    super::start_common1(
        start_kernel,
        end_kernel,
        &boot_info,
        stack_end,
        stack_size,
        unsafe { &memory::PAGE_DIRECTORY_BOOT1 as *const memory::PageTable as usize },
    );

    {
        let mut idt = INTERRUPT_DESCRIPTOR_TABLE.sync_lock();
        unsafe {
            idt[0].set_handler_addr(x86_64::addr::VirtAddr::from_ptr(
                divide_by_zero as *const (),
            ));
            idt.segment_not_present.set_handler_fn(segment_not_present);

            let mut entry = x86_64::structures::idt::Entry::missing();
            entry.set_handler_addr(x86_64::addr::VirtAddr::from_ptr(
                double_fault_handler as *const (),
            ));
            idt.double_fault = entry;

            idt.general_protection_fault
                .set_handler_fn(general_protection_handler);

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

            let mut entry = x86_64::structures::idt::Entry::missing();
            entry.set_handler_addr(x86_64::addr::VirtAddr::from_ptr(
                breakpoint_exception as *const (),
            ));
            idt.breakpoint = entry;
            let mut entry = x86_64::structures::idt::Entry::missing();
            entry.set_handler_addr(x86_64::addr::VirtAddr::from_ptr(
                debug_exception as *const (),
            ));
            idt.debug = entry;
            idt[0x20].set_handler_fn(irq0);
            idt[0x21].set_handler_fn(irq1);
            idt[0x23].set_handler_fn(irq3);
            idt[0x24].set_handler_fn(irq4);
            idt[0x27].set_handler_fn(irq7);
            idt[0x29].set_handler_fn(irq9);
            idt[0x2a].set_handler_fn(irq10);
            idt[0x2b].set_handler_fn(irq11);
            idt[0x2c].set_handler_fn(irq12);
            idt[0x2f].set_handler_fn(irq15);
        }
    }

    let mut sys = {
        doors_macros::config_build_struct! {
            X86System {
                boot_info,
                #[doorsconfig = "acpi"]
                acpi_handler: super::Acpi {
                    pageman: &PAGING_MANAGER,
                    vmm: &VIRTUAL_MEMORY_ALLOCATOR,
                },
                cpuid,
                acpi: None,
                stack_start: stack_end - stack_size,
            }
        }
    };
    sys.load_acpi();
    unsafe {
        INTERRUPT_DESCRIPTOR_TABLE.sync_lock().load_unsafe();
    }

    *crate::SYSTEM.write() = kernel::System::X86_64(LockedArc::new(sys));
    super::main_boot();
}

//! This is the 64 bit module for x86 hardware. It contains the entry point for the 64-bit kernnel on x86.

use crate::kernel;
use crate::modules::interrupt::InterruptControllerTrait;
use crate::Locked;
use crate::LockedArc;
use alloc::boxed::Box;
use core::alloc::Allocator;
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
    let p = crate::kernel::INTERRUPT_CONTROLLER.read();
    if let Some(p) = p.as_ref() {
        p.end_of_interrupt(irqnum)
    }
}

/// The generic irq handler
pub extern "x86-interrupt" fn generic_irq<const T: u8>(_isf: InterruptStackFrame) {
    let handle = super::IRQ_HANDLERS[T as usize].sync_lock();
    let h3 = unsafe { handle.unsafe_destroy() };
    let h3 = unsafe { h3.as_mut().unwrap() };
    finish_irq(T);
    if let Some(h2) = h3 {
        h2();
    }
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
    Locked::new(memory::BumpAllocator::new(0x0000));

/// The physical memory manager for the system
pub static PAGE_ALLOCATOR: Locked<memory::SimpleMemoryManager> =
    Locked::new(memory::SimpleMemoryManager::new(&VIRTUAL_MEMORY_ALLOCATOR));

/// The paging manager, which controls the memory management unit. Responsible for mapping virtual memory addresses to physical addresses.
pub static PAGING_MANAGER: Locked<memory::PagingTableManager> =
    Locked::new(memory::PagingTableManager::new(&PAGE_ALLOCATOR));

/// The interrupt descriptor table for the system
pub static INTERRUPT_DESCRIPTOR_TABLE: Locked<InterruptDescriptorTable> =
    Locked::new(InterruptDescriptorTable::new());

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
pub struct X86System<'a> {
    /// Used for information regarding the bootup of the kernel
    pub boot_info: multiboot2::BootInformation<'a>,
    #[doorsconfig = "acpi"]
    /// Used for acpi
    pub acpi: Option<AcpiStuff>,
    #[doorsconfig = "acpi"]
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

    fn get_software_interrupt<F: FnMut() + Send + Sync + crate::Interrupt + 'static>(
        &self,
        h: F,
    ) -> Option<kernel::SoftwareInterrupt> {
        for i in (0..224).rev() {
            let mut handle = super::IRQ_HANDLERS[i as usize].sync_lock();
            if handle.is_none() {
                let h = Box::new(h);
                *handle = Some(h);
                return Some(kernel::SoftwareInterrupt::X86_64(
                    crate::kernel::x86::SoftIrq { irq: i },
                ));
            }
        }
        None
    }

    fn enable_irq(&self, irq: u8) {
        self.disable_interrupts_for(|| {
            let p = crate::kernel::INTERRUPT_CONTROLLER.read();
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
    ) -> bool {
        let a = Box::new(handler);
        let mut irqs = super::IRQ_HANDLERS[irq as usize].sync_lock();
        if irqs.is_none() {
            irqs.replace(a);
            true
        } else {
            false
        }
    }

    fn breakpoint(&self) -> Option<u8> {
        Some(0xcc)
    }

    unsafe fn unregister_irq_handler(&self, irq: u8) {
        let mut irqs = super::IRQ_HANDLERS[irq as usize].sync_lock();
        irqs.take();
    }

    unsafe fn unregister_exception_handler(&self, exception: u8) {
        let mut irqs = super::EXCEPTION_HANDLERS[exception as usize].sync_lock();
        irqs.take();
    }

    fn register_exception_handler<F: FnMut() + Send + Sync + crate::Interrupt + 'static>(
        &self,
        exception: u8,
        handler: F,
    ) -> bool {
        let a = Box::new(handler);
        let mut irqs = super::EXCEPTION_HANDLERS[exception as usize].sync_lock();
        if irqs.is_none() {
            irqs.replace(a);
            true
        } else {
            false
        }
    }

    fn disable_irq(&self, irq: u8) {
        self.disable_interrupts_for(|| {
            let p = crate::kernel::INTERRUPT_CONTROLLER.read();
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
}

/// The entry point for the 64 bit x86 kernel
#[no_mangle]
pub extern "C" fn start64() -> ! {
    // Early debug output - write directly to VGA memory to show we reached Rust code
    unsafe {
        let vga = 0xb8000 as *mut u8;
        *vga.add(2) = b'R';
        *vga.add(3) = 0x07;
    }

    let cpuid = raw_cpuid::CpuId::new();

    // Debug: Show we got past cpuid
    unsafe {
        let vga = 0xb8000 as *mut u8;
        *vga.add(4) = b'1';
        *vga.add(5) = 0x07;
    }

    let start_kernel = unsafe { &super::START_OF_KERNEL } as *const u8 as usize;
    let end_kernel = unsafe { &super::END_OF_KERNEL } as *const u8 as usize;

    // Debug: Show we got kernel addresses

    let a = unsafe { &memory::TABLE4 } as *const u64 as usize;
    let b = unsafe { &memory::TABLE3 } as *const u64 as usize;
    let c = unsafe { &memory::TABLE2 } as *const u64 as usize;
    let d = unsafe { &memory::TABLE1 } as *const u64 as usize;
    let page_entries = [
        memory::PageTableModifierData {
            virt: 0x400000,
            entry: a,
        },
        memory::PageTableModifierData {
            virt: 0x401000,
            entry: b,
        },
        memory::PageTableModifierData {
            virt: 0x402000,
            entry: c,
        },
        memory::PageTableModifierData {
            virt: 0x403000,
            entry: d,
        },
    ];

    // Debug: Show we set up page entries
    unsafe {
        let vga = 0xb8000 as *mut u8;
        *vga.add(6) = b'2';
        *vga.add(7) = 0x07;
    }

    //Copy the boot information header to the end of the kernel, update the end of the kernel variable to reflect the new data
    let bi_size = {
        let boot_info = unsafe {
            // Debug: Show multiboot2 load attempt
            let vga = 0xb8000 as *mut u8;
            *vga.add(8) = b'M';
            *vga.add(9) = 0x07;

            match multiboot2::BootInformation::load(
                super::MULTIBOOT2_DATA as *const multiboot2::BootInformationHeader,
            ) {
                Ok(info) => {
                    // Debug: Show multiboot2 load success
                    *vga.add(10) = b'O';
                    *vga.add(11) = 0x07;
                    info
                }
                Err(_) => {
                    // Debug: Show multiboot2 load failed
                    *vga.add(10) = b'E';
                    *vga.add(11) = 0x07;
                    loop {
                        core::arch::asm!("hlt");
                    }
                }
            }
        };

        let size = boot_info.total_size();

        // Debug: Show we got the size
        unsafe {
            let vga = 0xb8000 as *mut u8;
            *vga.add(12) = b'Z';
            *vga.add(13) = 0x07;
        }

        // Align end_kernel to 8-byte boundary for multiboot2 data
        let aligned_end_kernel = (end_kernel + 7) & !7;

        let dest = unsafe { core::slice::from_raw_parts_mut(aligned_end_kernel as *mut u8, size) };

        // Debug: Show we created dest slice
        unsafe {
            let vga = 0xb8000 as *mut u8;
            *vga.add(14) = b'D';
            *vga.add(15) = 0x07;
        }

        // Use original MULTIBOOT2_DATA address as source, not boot_info.start_address()
        let source =
            unsafe { core::slice::from_raw_parts(super::MULTIBOOT2_DATA as *const u8, size) };

        // Debug: Show we created source slice
        unsafe {
            let vga = 0xb8000 as *mut u8;
            *vga.add(16) = b'T';
            *vga.add(17) = 0x07;
        }

        // Handle overlapping memory regions properly
        let source_mut =
            unsafe { core::slice::from_raw_parts_mut(super::MULTIBOOT2_DATA as *mut u8, size) };

        if crate::slice_address(dest) < crate::slice_address(source_mut) {
            let di = dest.iter_mut();
            let si = source_mut.iter();
            let a = si.zip(di);
            for (s, d) in a {
                *d = *s;
            }
        } else {
            // For overlapping memory, copy from end backwards
            for i in (0..size).rev() {
                dest[i] = source_mut[i];
            }
        }

        // Debug: Show copy completed
        unsafe {
            let vga = 0xb8000 as *mut u8;
            *vga.add(18) = b'P';
            *vga.add(19) = 0x07;
        }

        size
    };

    // Debug: Show about to load from copied location
    unsafe {
        let vga = 0xb8000 as *mut u8;
        *vga.add(20) = b'L';
        *vga.add(21) = 0x07;
    }

    let boot_info = unsafe {
        let aligned_end_kernel = (end_kernel + 7) & !7;

        match multiboot2::BootInformation::load(
            aligned_end_kernel as *const multiboot2::BootInformationHeader,
        ) {
            Ok(info) => {
                // Debug: Show second multiboot2 load success
                let vga = 0xb8000 as *mut u8;
                *vga.add(22) = b'N';
                *vga.add(23) = 0x07;
                info
            }
            Err(_) => {
                // Debug: Show second multiboot2 load failed
                let vga = 0xb8000 as *mut u8;
                *vga.add(22) = b'F';
                *vga.add(23) = 0x07;
                loop {
                    core::arch::asm!("hlt");
                }
            }
        }
    };

    let end_kernel = (end_kernel + 7) & !7; // Use aligned address
    let end_kernel = end_kernel + bi_size;

    // Debug: Show about to call start_common1
    unsafe {
        let vga = 0xb8000 as *mut u8;
        *vga.add(24) = b'S';
        *vga.add(25) = 0x07;
    }

    let stack_end = unsafe { super::INITIAL_STACK as usize };
    let stack_size = MAIN_STACK_SIZE as usize;

    super::start_common1(
        start_kernel,
        end_kernel,
        &boot_info,
        stack_end,
        stack_size,
        unsafe { &memory::PAGE_DIRECTORY_BOOT1 as *const memory::PageTable as usize },
        &page_entries,
    );

    // Debug: Show start_common1 completed
    unsafe {
        let vga = 0xb8000 as *mut u8;
        *vga.add(26) = b'C';
        *vga.add(27) = 0x07;
    }

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
            idt[0x20].set_handler_fn(generic_irq::<0>);
            idt[0x21].set_handler_fn(generic_irq::<1>);
            idt[0x22].set_handler_fn(generic_irq::<2>);
            idt[0x23].set_handler_fn(generic_irq::<3>);
            idt[0x24].set_handler_fn(generic_irq::<4>);
            idt[0x25].set_handler_fn(generic_irq::<5>);
            idt[0x26].set_handler_fn(generic_irq::<6>);
            idt[0x27].set_handler_fn(generic_irq::<7>);
            idt[0x28].set_handler_fn(generic_irq::<8>);
            idt[0x29].set_handler_fn(generic_irq::<9>);
            idt[0x2a].set_handler_fn(generic_irq::<10>);
            idt[0x2b].set_handler_fn(generic_irq::<11>);
            idt[0x2c].set_handler_fn(generic_irq::<12>);
            idt[0x2d].set_handler_fn(generic_irq::<13>);
            idt[0x2e].set_handler_fn(generic_irq::<14>);
            idt[0x2f].set_handler_fn(generic_irq::<15>);
            idt[0x30].set_handler_fn(generic_irq::<16>);
            idt[0x31].set_handler_fn(generic_irq::<17>);
            idt[0x32].set_handler_fn(generic_irq::<18>);
            idt[0x33].set_handler_fn(generic_irq::<19>);
            idt[0x34].set_handler_fn(generic_irq::<20>);
            idt[0x35].set_handler_fn(generic_irq::<21>);
            idt[0x36].set_handler_fn(generic_irq::<22>);
            idt[0x37].set_handler_fn(generic_irq::<23>);
            idt[0x38].set_handler_fn(generic_irq::<24>);
            idt[0x39].set_handler_fn(generic_irq::<25>);
            idt[0x3a].set_handler_fn(generic_irq::<26>);
            idt[0x3b].set_handler_fn(generic_irq::<27>);
            idt[0x3c].set_handler_fn(generic_irq::<28>);
            idt[0x3d].set_handler_fn(generic_irq::<29>);
            idt[0x3e].set_handler_fn(generic_irq::<30>);
            idt[0x3f].set_handler_fn(generic_irq::<31>);
            idt[0x40].set_handler_fn(generic_irq::<32>);
            idt[0x41].set_handler_fn(generic_irq::<33>);
            idt[0x42].set_handler_fn(generic_irq::<34>);
            idt[0x43].set_handler_fn(generic_irq::<35>);
            idt[0x44].set_handler_fn(generic_irq::<36>);
            idt[0x45].set_handler_fn(generic_irq::<37>);
            idt[0x46].set_handler_fn(generic_irq::<38>);
            idt[0x47].set_handler_fn(generic_irq::<39>);
            idt[0x48].set_handler_fn(generic_irq::<40>);
            idt[0x49].set_handler_fn(generic_irq::<41>);
            idt[0x4a].set_handler_fn(generic_irq::<42>);
            idt[0x4b].set_handler_fn(generic_irq::<43>);
            idt[0x4c].set_handler_fn(generic_irq::<44>);
            idt[0x4d].set_handler_fn(generic_irq::<45>);
            idt[0x4e].set_handler_fn(generic_irq::<46>);
            idt[0x4f].set_handler_fn(generic_irq::<47>);
            idt[0x50].set_handler_fn(generic_irq::<48>);
            idt[0x51].set_handler_fn(generic_irq::<49>);
            idt[0x52].set_handler_fn(generic_irq::<50>);
            idt[0x53].set_handler_fn(generic_irq::<51>);
            idt[0x54].set_handler_fn(generic_irq::<52>);
            idt[0x55].set_handler_fn(generic_irq::<53>);
            idt[0x56].set_handler_fn(generic_irq::<54>);
            idt[0x57].set_handler_fn(generic_irq::<55>);
            idt[0x58].set_handler_fn(generic_irq::<56>);
            idt[0x59].set_handler_fn(generic_irq::<57>);
            idt[0x5a].set_handler_fn(generic_irq::<58>);
            idt[0x5b].set_handler_fn(generic_irq::<59>);
            idt[0x5c].set_handler_fn(generic_irq::<60>);
            idt[0x5d].set_handler_fn(generic_irq::<61>);
            idt[0x5e].set_handler_fn(generic_irq::<62>);
            idt[0x5f].set_handler_fn(generic_irq::<63>);
            idt[0x60].set_handler_fn(generic_irq::<64>);
            idt[0x61].set_handler_fn(generic_irq::<65>);
            idt[0x62].set_handler_fn(generic_irq::<66>);
            idt[0x63].set_handler_fn(generic_irq::<67>);
            idt[0x64].set_handler_fn(generic_irq::<68>);
            idt[0x65].set_handler_fn(generic_irq::<69>);
            idt[0x66].set_handler_fn(generic_irq::<70>);
            idt[0x67].set_handler_fn(generic_irq::<71>);
            idt[0x68].set_handler_fn(generic_irq::<72>);
            idt[0x69].set_handler_fn(generic_irq::<73>);
            idt[0x6a].set_handler_fn(generic_irq::<74>);
            idt[0x6b].set_handler_fn(generic_irq::<75>);
            idt[0x6c].set_handler_fn(generic_irq::<76>);
            idt[0x6d].set_handler_fn(generic_irq::<77>);
            idt[0x6e].set_handler_fn(generic_irq::<78>);
            idt[0x6f].set_handler_fn(generic_irq::<79>);
            idt[0x70].set_handler_fn(generic_irq::<80>);
            idt[0x71].set_handler_fn(generic_irq::<81>);
            idt[0x72].set_handler_fn(generic_irq::<82>);
            idt[0x73].set_handler_fn(generic_irq::<83>);
            idt[0x74].set_handler_fn(generic_irq::<84>);
            idt[0x75].set_handler_fn(generic_irq::<85>);
            idt[0x76].set_handler_fn(generic_irq::<86>);
            idt[0x77].set_handler_fn(generic_irq::<87>);
            idt[0x78].set_handler_fn(generic_irq::<88>);
            idt[0x79].set_handler_fn(generic_irq::<89>);
            idt[0x7a].set_handler_fn(generic_irq::<90>);
            idt[0x7b].set_handler_fn(generic_irq::<91>);
            idt[0x7c].set_handler_fn(generic_irq::<92>);
            idt[0x7d].set_handler_fn(generic_irq::<93>);
            idt[0x7e].set_handler_fn(generic_irq::<94>);
            idt[0x7f].set_handler_fn(generic_irq::<95>);
            idt[0x80].set_handler_fn(generic_irq::<96>);
            idt[0x81].set_handler_fn(generic_irq::<97>);
            idt[0x82].set_handler_fn(generic_irq::<98>);
            idt[0x83].set_handler_fn(generic_irq::<99>);
            idt[0x84].set_handler_fn(generic_irq::<100>);
            idt[0x85].set_handler_fn(generic_irq::<101>);
            idt[0x86].set_handler_fn(generic_irq::<102>);
            idt[0x87].set_handler_fn(generic_irq::<103>);
            idt[0x88].set_handler_fn(generic_irq::<104>);
            idt[0x89].set_handler_fn(generic_irq::<105>);
            idt[0x8a].set_handler_fn(generic_irq::<106>);
            idt[0x8b].set_handler_fn(generic_irq::<107>);
            idt[0x8c].set_handler_fn(generic_irq::<108>);
            idt[0x8d].set_handler_fn(generic_irq::<109>);
            idt[0x8e].set_handler_fn(generic_irq::<110>);
            idt[0x8f].set_handler_fn(generic_irq::<111>);
            idt[0x90].set_handler_fn(generic_irq::<112>);
            idt[0x91].set_handler_fn(generic_irq::<113>);
            idt[0x92].set_handler_fn(generic_irq::<114>);
            idt[0x93].set_handler_fn(generic_irq::<115>);
            idt[0x94].set_handler_fn(generic_irq::<116>);
            idt[0x95].set_handler_fn(generic_irq::<117>);
            idt[0x96].set_handler_fn(generic_irq::<118>);
            idt[0x97].set_handler_fn(generic_irq::<119>);
            idt[0x98].set_handler_fn(generic_irq::<120>);
            idt[0x99].set_handler_fn(generic_irq::<121>);
            idt[0x9a].set_handler_fn(generic_irq::<122>);
            idt[0x9b].set_handler_fn(generic_irq::<123>);
            idt[0x9c].set_handler_fn(generic_irq::<124>);
            idt[0x9d].set_handler_fn(generic_irq::<125>);
            idt[0x9e].set_handler_fn(generic_irq::<126>);
            idt[0x9f].set_handler_fn(generic_irq::<127>);
            idt[0xa0].set_handler_fn(generic_irq::<128>);
            idt[0xa1].set_handler_fn(generic_irq::<129>);
            idt[0xa2].set_handler_fn(generic_irq::<130>);
            idt[0xa3].set_handler_fn(generic_irq::<131>);
            idt[0xa4].set_handler_fn(generic_irq::<132>);
            idt[0xa5].set_handler_fn(generic_irq::<133>);
            idt[0xa6].set_handler_fn(generic_irq::<134>);
            idt[0xa7].set_handler_fn(generic_irq::<135>);
            idt[0xa8].set_handler_fn(generic_irq::<136>);
            idt[0xa9].set_handler_fn(generic_irq::<137>);
            idt[0xaa].set_handler_fn(generic_irq::<138>);
            idt[0xab].set_handler_fn(generic_irq::<139>);
            idt[0xac].set_handler_fn(generic_irq::<140>);
            idt[0xad].set_handler_fn(generic_irq::<141>);
            idt[0xae].set_handler_fn(generic_irq::<142>);
            idt[0xaf].set_handler_fn(generic_irq::<143>);
            idt[0xb0].set_handler_fn(generic_irq::<144>);
            idt[0xb1].set_handler_fn(generic_irq::<145>);
            idt[0xb2].set_handler_fn(generic_irq::<146>);
            idt[0xb3].set_handler_fn(generic_irq::<147>);
            idt[0xb4].set_handler_fn(generic_irq::<148>);
            idt[0xb5].set_handler_fn(generic_irq::<149>);
            idt[0xb6].set_handler_fn(generic_irq::<150>);
            idt[0xb7].set_handler_fn(generic_irq::<151>);
            idt[0xb8].set_handler_fn(generic_irq::<152>);
            idt[0xb9].set_handler_fn(generic_irq::<153>);
            idt[0xba].set_handler_fn(generic_irq::<154>);
            idt[0xbb].set_handler_fn(generic_irq::<155>);
            idt[0xbc].set_handler_fn(generic_irq::<156>);
            idt[0xbd].set_handler_fn(generic_irq::<157>);
            idt[0xbe].set_handler_fn(generic_irq::<158>);
            idt[0xbf].set_handler_fn(generic_irq::<159>);
            idt[0xc0].set_handler_fn(generic_irq::<160>);
            idt[0xc1].set_handler_fn(generic_irq::<161>);
            idt[0xc2].set_handler_fn(generic_irq::<162>);
            idt[0xc3].set_handler_fn(generic_irq::<163>);
            idt[0xc4].set_handler_fn(generic_irq::<164>);
            idt[0xc5].set_handler_fn(generic_irq::<165>);
            idt[0xc6].set_handler_fn(generic_irq::<166>);
            idt[0xc7].set_handler_fn(generic_irq::<167>);
            idt[0xc8].set_handler_fn(generic_irq::<168>);
            idt[0xc9].set_handler_fn(generic_irq::<169>);
            idt[0xca].set_handler_fn(generic_irq::<170>);
            idt[0xcb].set_handler_fn(generic_irq::<171>);
            idt[0xcc].set_handler_fn(generic_irq::<172>);
            idt[0xcd].set_handler_fn(generic_irq::<173>);
            idt[0xce].set_handler_fn(generic_irq::<174>);
            idt[0xcf].set_handler_fn(generic_irq::<175>);
            idt[0xd0].set_handler_fn(generic_irq::<176>);
            idt[0xd1].set_handler_fn(generic_irq::<177>);
            idt[0xd2].set_handler_fn(generic_irq::<178>);
            idt[0xd3].set_handler_fn(generic_irq::<179>);
            idt[0xd4].set_handler_fn(generic_irq::<180>);
            idt[0xd5].set_handler_fn(generic_irq::<181>);
            idt[0xd6].set_handler_fn(generic_irq::<182>);
            idt[0xd7].set_handler_fn(generic_irq::<183>);
            idt[0xd8].set_handler_fn(generic_irq::<184>);
            idt[0xd9].set_handler_fn(generic_irq::<185>);
            idt[0xda].set_handler_fn(generic_irq::<186>);
            idt[0xdb].set_handler_fn(generic_irq::<187>);
            idt[0xdc].set_handler_fn(generic_irq::<188>);
            idt[0xdd].set_handler_fn(generic_irq::<189>);
            idt[0xde].set_handler_fn(generic_irq::<190>);
            idt[0xdf].set_handler_fn(generic_irq::<191>);
            idt[0xe0].set_handler_fn(generic_irq::<192>);
            idt[0xe1].set_handler_fn(generic_irq::<193>);
            idt[0xe2].set_handler_fn(generic_irq::<194>);
            idt[0xe3].set_handler_fn(generic_irq::<195>);
            idt[0xe4].set_handler_fn(generic_irq::<196>);
            idt[0xe5].set_handler_fn(generic_irq::<197>);
            idt[0xe6].set_handler_fn(generic_irq::<198>);
            idt[0xe7].set_handler_fn(generic_irq::<199>);
            idt[0xe8].set_handler_fn(generic_irq::<200>);
            idt[0xe9].set_handler_fn(generic_irq::<201>);
            idt[0xea].set_handler_fn(generic_irq::<202>);
            idt[0xeb].set_handler_fn(generic_irq::<203>);
            idt[0xec].set_handler_fn(generic_irq::<204>);
            idt[0xed].set_handler_fn(generic_irq::<205>);
            idt[0xee].set_handler_fn(generic_irq::<206>);
            idt[0xef].set_handler_fn(generic_irq::<207>);
            idt[0xf0].set_handler_fn(generic_irq::<208>);
            idt[0xf1].set_handler_fn(generic_irq::<209>);
            idt[0xf2].set_handler_fn(generic_irq::<210>);
            idt[0xf3].set_handler_fn(generic_irq::<211>);
            idt[0xf4].set_handler_fn(generic_irq::<212>);
            idt[0xf5].set_handler_fn(generic_irq::<213>);
            idt[0xf6].set_handler_fn(generic_irq::<214>);
            idt[0xf7].set_handler_fn(generic_irq::<215>);
            idt[0xf8].set_handler_fn(generic_irq::<216>);
            idt[0xf9].set_handler_fn(generic_irq::<217>);
            idt[0xfa].set_handler_fn(generic_irq::<218>);
            idt[0xfb].set_handler_fn(generic_irq::<219>);
            idt[0xfc].set_handler_fn(generic_irq::<220>);
            idt[0xfd].set_handler_fn(generic_irq::<221>);
            idt[0xfe].set_handler_fn(generic_irq::<222>);
            idt[0xff].set_handler_fn(generic_irq::<223>);
        }
    }

    let mut sys = {
        doors_macros::config_build_struct! {
            X86System {
                boot_info,
                #[doorsconfig = "acpi"]
                acpi: Some(AcpiStuff::Handler(super::Acpi {
                    pageman: &PAGING_MANAGER,
                    vmm: &VIRTUAL_MEMORY_ALLOCATOR,
                })),
                cpuid,
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

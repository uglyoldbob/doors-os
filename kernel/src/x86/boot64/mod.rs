//! This is the 64 bit module for x86 hardware. It contains the entry point for the 64-bit kernnel on x86.

use doors_macros::interrupt_64;
use doors_macros::interrupt_arg_64;
use lazy_static::lazy_static;

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
    let gdt = GlobalDescriptorTable::new();
    gdt.const_add_entry(Descriptor::kernel_code_segment())
        .const_add_entry(Descriptor::kernel_data_segment())
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

use doors_kernel_api::video::TextDisplay;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = build_idt();
    static ref APIC: spin::Mutex<X86Apic> = spin::Mutex::new(unsafe { X86Apic::get() });
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

/// The entry point for the 64 bit x86 kernel
#[no_mangle]
pub extern "C" fn start64() -> ! {
    super::VGA.lock().print_str(GREETING);
    let cpuid = raw_cpuid::CpuId::new();

    unsafe {
        IDT.load_unsafe();
        //x86_64::instructions::interrupts::enable();
    }
    super::main_boot();
}

//! The generic x86 module covering both 32 and 64-bit functionality.

use crate::modules::video::text::X86VgaTextMode;
use doors_kernel_api::video::TextDisplay;
use lazy_static::lazy_static;

#[cfg(target_arch = "x86_64")]
pub mod boot64;
#[cfg(target_arch = "x86_64")]
pub use boot64 as boot;

#[cfg(target_arch = "x86")]
pub mod boot32;
#[cfg(target_arch = "x86")]
pub use boot32 as boot;

pub mod memory;

lazy_static! {
    /// The VGA instance used for x86 kernel printing
    static ref VGA: spin::Mutex<X86VgaTextMode<'static>> =
        spin::Mutex::new(unsafe { X86VgaTextMode::get(0xb8000) });
    static ref IOPORTS: spin::Mutex<bitarray::BitArray<65536>> =
        spin::Mutex::new(bitarray::BitArray::new([0; 65536]));
}

/// The heap for the kernel. This global allocator is responsible for the majority of dynamic memory in the kernel.
#[global_allocator]
static HEAP_MANAGER: crate::Locked<memory::HeapManager> = crate::Locked::new(
    memory::HeapManager::new(&boot::PAGING_MANAGER, &boot::VIRTUAL_MEMORY_ALLOCATOR),
);

/// This function is called by the entrance module for the kernel.
fn main_boot() -> ! {
    VGA.lock().print_str("main boot\r\n");
    super::super::main(&*VGA);
}

//! Boot code for the stm32F769i-disco development board

pub mod memory;

/// The entry point of the kernel
#[no_mangle]
pub extern "C" fn _start() -> ! {
    crate::main()
}

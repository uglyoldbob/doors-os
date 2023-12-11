//! Boot code for arm based platforms

use crate::Locked;

/// The panic handler
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

/// The heap for the kernel. This global allocator is responsible for the majority of dynamic memory in the kernel.
#[global_allocator]
static HEAP_MANAGER: Locked<hardware::memory::HeapManager> =
    Locked::new(hardware::memory::HeapManager::new());

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769i_disco;

#[cfg(kernel_machine = "stm32f769i-disco")]
pub use stm32f769i_disco as hardware;

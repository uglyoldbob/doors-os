//! Boot code for arm based platforms

use crate::Locked;

use crate::modules::video::TextDisplayTrait;

/// Attempt to get a str from a panic payload
pub fn get_panic_message(panic: &dyn core::any::Any) -> Option<&str> {
    panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().map(core::ops::Deref::deref))
}

/// The heap for the kernel. This global allocator is responsible for the majority of dynamic memory in the kernel.
#[global_allocator]
static HEAP_MANAGER: Locked<hardware::memory::HeapManager> =
    Locked::new(hardware::memory::HeapManager::new());

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769i_disco;

use alloc::string::String;
#[cfg(kernel_machine = "stm32f769i-disco")]
pub use stm32f769i_disco as hardware;

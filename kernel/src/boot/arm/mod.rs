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

/// The panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    doors_macros2::kernel_print!("PANIC AT THE DISCO!\r\n");

    let payload = info.payload();
    let message = get_panic_message(payload);
    if let Some(m) = message {
        doors_macros2::kernel_print!("MESSAGE:\r\n");
        doors_macros2::kernel_print!("MESSAGE: {:?}\r\n", m);
    }

    if let Some(t) = info.location() {
        let f = t.file();
        let maxlen = f.len();
        for i in (0..maxlen).step_by(70) {
            let tmax = if i + 70 < maxlen { i + 70} else { maxlen};
            doors_macros2::kernel_print!("{}\r\n", &f[i..tmax]);
        }
        doors_macros2::kernel_print!(" LINE {}\r\n", t.line());
    }
    doors_macros2::kernel_print!("PANIC SOMEWHERE ELSE!\r\n");
    loop {
    }
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

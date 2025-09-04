//! This is a test binary for the kernel

#![no_std]
#![no_main]
#![allow(async_fn_in_trait)]
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![feature(allocator_api)]
#![feature(abi_x86_interrupt)]
#![feature(async_fn_traits)]
#![feature(auto_traits)]
#![feature(negative_impls)]
#![feature(type_alias_impl_trait)]

/// The panic handler for the kernel
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    loop {}
}

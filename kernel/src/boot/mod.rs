//! This module contains architecture specific boot code.

#[cfg(target_arch = "arm")]
pub mod arm;
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod multiboot;
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod x86;
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub use x86::Allocator;
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub use x86::NON_RAM_ALLOCATOR;

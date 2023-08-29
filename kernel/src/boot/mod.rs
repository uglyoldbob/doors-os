//! This module contains architecture specific boot code.

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod multiboot;
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod x86;
//! This module contains architecture specific boot code.

use crate::boot::x86::boot64::PAGING_MANAGER;

/// The trait for memory managers that need to ensure that virtual memory is mapped to real physical memory.
#[enum_dispatch::enum_dispatch]
pub trait VirtualMemoryMapperTrait {
    /// Used to allocate physical memory to the given virtual address range
    fn allocate_physical_pages(&self, virt: usize, len: usize) -> Result<(), ()>;
    /// Used to unallocate physical memory from the given virtual address range
    fn unallocate_physical_pages(&self, virt: usize, len: usize) -> Result<(), ()>;
}

/// Maps and unmaps physical memory to virtual memory
#[enum_dispatch::enum_dispatch(VirtualMemoryMapperTrait)]
pub enum VirtualMemoryMapper<'a> {
    /// The 64 bit x86 variant
    #[cfg(target_arch = "x86_64")]
    X86Mapper(x86::boot::memory::VirtualMemoryMapper<'a>),
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "arm")] {
        pub mod arm;
    } else if #[cfg(any(target_arch = "x86_64", target_arch = "x86"))] {
        pub mod multiboot;
        pub mod x86;
        pub use x86::IoPortManager;
        pub use x86::IOPORTS;
        pub use x86::IoPortArray;
        pub use x86::IoPortRef;
        pub use x86::mem2;
        /// The virt mem mapper for the kernel
        pub static VIRT_MEM_MAPPER : VirtualMemoryMapper = VirtualMemoryMapper::X86Mapper(x86::boot::memory::VirtualMemoryMapper::new(&PAGING_MANAGER));
    }
}

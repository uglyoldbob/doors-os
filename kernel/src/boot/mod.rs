//! This module contains architecture specific boot code.

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
use crate::boot::x86::boot::PAGING_MANAGER;

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
    /// The 32 bit x86 variant
    #[cfg(target_arch = "x86")]
    X86Mapper(x86::boot::memory::VirtualMemoryMapper<'a>),
    /// A mapper that does nothing, for hardware without a memory management unit
    #[cfg(kernel_machine = "stm32f769i-disco")]
    NullMapper(NullMapper<'a>),
}

/// A mapper that does nothing, used on hardware without a memory management unit
#[cfg(kernel_machine = "stm32f769i-disco")]
struct NullMapper<'a> {
    _m: core::marker::PhantomData<&'a ()>,
}

#[cfg(kernel_machine = "stm32f769i-disco")]
impl VirtualMemoryMapperTrait for NullMapper<'_> {
    fn allocate_physical_pages(&self, _virt: usize, _len: usize) -> Result<(), ()> {
        // There is no mmu, all addresses are already physical addresses
        Ok(())
    }

    fn unallocate_physical_pages(&self, _virt: usize, _len: usize) -> Result<(), ()> {
        // There is no mmu, all addresses are already physical addresses
        Ok(())
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "arm")] {
        pub mod arm;
        pub use arm::IoPortManager;
        pub use arm::IoPortRef;
        pub use arm::IoPortArray;
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

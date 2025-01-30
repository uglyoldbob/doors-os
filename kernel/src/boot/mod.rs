//! This module contains architecture specific boot code.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "arm")] {
        pub mod arm;
    } else if #[cfg(any(target_arch = "x86_64", target_arch = "x86"))] {
        pub mod multiboot;
        pub mod x86;
        pub use x86::PciMemoryAllocator;
        pub use x86::PCI_MEMORY_ALLOCATOR;
        pub use x86::IoPortManager;
        pub use x86::IOPORTS;
        pub use x86::IoPortArray;
        pub use x86::IoPortRef;
    }
}

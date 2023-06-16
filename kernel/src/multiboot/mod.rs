//! This contains the structure definition for creating a multiboot2 signature.

/// The main structure to use for multi-boot kernels.
#[repr(C, align(64))]
pub struct Multiboot {
    /// Magic value required for multiboot compliance
    magic: u32,
    /// Architecture for multiboot
    arch: u32,
    /// Length of this header
    length: u32,
    /// Checksum of this header
    checksum: u32,
}

impl Multiboot {
    /// Construct a new instance of a multiboot header for a multiboot kernel.
    pub const fn new() -> Self {
        let mut o = Self {
            magic: 0xe85250d6,
            arch: 0,
            length: 0x10,
            checksum: 0,
        };
        o.checksum = 0xFFFFFFFF - o.magic - o.arch - o.length + 1;
        o
    }
}

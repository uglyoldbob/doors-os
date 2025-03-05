//! This module defines a global descriptor table for 32-bit x86.

/// This structure defines the 32 bit global descriptor table
pub struct GlobalDescriptorTable {
    /// The entries of the global descriptor table. The first entry must be null.
    entries: [x86::segmentation::Descriptor; 8],
    /// The size of the global descriptor table in entries.
    len: usize,
}

impl GlobalDescriptorTable {
    /// Create a blank instance of the global descriptor table
    pub const fn new() -> Self {
        Self {
            entries: [x86::segmentation::Descriptor::NULL; 8],
            len: 1,
        }
    }

    /// Returns the number of entries occupied.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns the limit for the gdt
    pub const fn limit(&self) -> u16 {
        self.len as u16 * 8 - 1
    }

    /// Add an entry to the global descriptor table.
    pub const fn const_add_entry(&mut self, g: x86::segmentation::Descriptor) {
        self.entries[self.len] = g;
        self.len += 1;
    }
}

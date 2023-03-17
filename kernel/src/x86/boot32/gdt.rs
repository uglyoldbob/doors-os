/// This structure defines the 32 bit global descriptor table
pub struct GlobalDescriptorTable {
    entries: [x86::segmentation::Descriptor; 8],
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

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn const_add_entry(mut self, g: x86::segmentation::Descriptor) -> Self {
        self.entries[self.len] = g;
        self.len += 1;
        self
    }
}
//! Generic memory code (to be included from architecture specific memory code and re-exported)

/// A struct that manages allocation and deallocation of pci memory
pub struct PciMemory {
    /// The starting address for virtual memory address space
    virt: usize,
    /// The starting address for physical memory address space
    phys: usize,
    /// The size in bytes
    size: usize,
}

impl PciMemory {
    /// Construct a new instance. Should only be used in the memory management code!
    /// # Safety
    /// virt should be mapped to phys over a length of size
    /// this mapping should not be changed over the life of this object
    pub(super) unsafe fn build_with(virt: usize, phys: usize, size: usize) -> Self {
        Self { virt, phys, size }
    }

    /// Read a u32 at the specified index (byte based index)
    pub fn read_u32(&self, address: usize) -> u32 {
        let mem = unsafe { core::slice::from_raw_parts(self.virt as *const u8, self.size) };
        let a: &u8 = &mem[address];
        let b: *const u8 = a as *const u8;
        let c: &u32 = unsafe { &*(b as *const u32) };
        unsafe { core::ptr::read_volatile(c) }
    }

    /// Write a u32 at the specified index (byte based index), with the specified value
    pub fn write_u32(&mut self, address: usize, val: u32) {
        let mem = unsafe { core::slice::from_raw_parts_mut(self.virt as *mut u8, self.size) };
        let a: &mut u8 = &mut mem[address];
        let b: *mut u8 = a as *mut u8;
        let c: &mut u32 = unsafe { &mut *(b as *mut u32) };
        unsafe { core::ptr::write_volatile(c, val) };
    }

    /// Get the size of the memory area in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the starting physical address for the region
    pub fn phys(&self) -> usize {
        self.phys
    }

    /// Get the starting virtual address for the region
    pub fn virt(&self) -> usize {
        self.virt
    }
}

/// A structure that generically maps dma memory over a type.
pub struct DmaMemory<T> {
    /// The starting address for virtual memory address space
    virt: usize,
    /// The starting address for physical memory address space
    phys: usize,
    /// The size in bytes
    size: usize,
    /// The data (in virtual memory space)
    data: alloc::boxed::Box<T>,
}

impl<T> DmaMemory<T> {
    /// Construct a new instance. Should only be used in the memory management code!
    /// # Safety
    /// virt should be mapped to phys over a length of size
    /// this mapping should not be changed over the life of this object
    pub(super) unsafe fn build_with(
        virt: usize,
        phys: usize,
        size: usize,
        data: alloc::boxed::Box<T>,
    ) -> Self {
        Self {
            virt,
            phys,
            size,
            data,
        }
    }

    /// Get the size of the memory area in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the starting physical address for the region
    pub fn phys(&self) -> usize {
        self.phys
    }

    /// Get the starting virtual address for the region
    pub fn virt(&self) -> usize {
        self.virt
    }
}

impl<T> core::ops::Deref for DmaMemory<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> core::ops::DerefMut for DmaMemory<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

/// Used to store an array of items for dma
pub struct DmaMemorySlice<T> {
    /// The starting address for virtual memory address space
    virt: usize,
    /// The starting address for physical memory address space
    phys: usize,
    /// The size in bytes
    size: usize,
    /// The data (in virtual memory space)
    data: alloc::vec::Vec<T>,
}

impl<T> DmaMemorySlice<T> {
    /// Construct a new instance. Should only be used in the memory management code!
    /// # Safety
    /// virt should be mapped to phys over a length of size
    /// this mapping should not be changed over the life of this object
    pub(super) unsafe fn build_with(
        virt: usize,
        phys: usize,
        size: usize,
        data: alloc::vec::Vec<T>,
    ) -> Self {
        Self {
            virt,
            phys,
            size,
            data,
        }
    }

    /// Get the size of the memory area in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the starting physical address for the region
    pub fn phys(&self) -> usize {
        self.phys
    }

    /// Get the starting virtual address for the region
    pub fn virt(&self) -> usize {
        self.virt
    }
}

impl<T> core::ops::Deref for DmaMemorySlice<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> core::ops::DerefMut for DmaMemorySlice<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

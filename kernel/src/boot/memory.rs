//! Generic memory code (to be included from architecture specific memory code and re-exported)

use crate::Locked;

use super::BumpAllocator;

use core::{marker::PhantomData, mem::MaybeUninit};

use alloc::{alloc::Allocator, boxed::Box};

/// A destructure form of physical memory
pub struct DestructuredPhysicalMemory<T> {
    /// Start address
    addr: usize,
    /// phantom to pretend this holds a type T
    phantom: PhantomData<T>,
}

impl<T> DestructuredPhysicalMemory<T> {
    /// Returns the physical address
    pub fn address(&self) -> usize {
        self.addr
    }

    /// Rebuild the original box with physical memory.
    /// # Safety - You must use the same allocator as originally used to create the box.
    pub unsafe fn rebuild(self, mm: &BumpAllocator) -> Box<MaybeUninit<T>, &BumpAllocator> {
        let raw = self.addr as *mut MaybeUninit<T>;
        Box::<MaybeUninit<T>, &BumpAllocator>::from_raw_in(raw, mm)
    }
}

impl<'a, T> From<Box<MaybeUninit<T>, &Locked<super::SimpleMemoryManager<'a>>>>
    for DestructuredPhysicalMemory<T>
{
    fn from(value: Box<MaybeUninit<T>, &Locked<super::SimpleMemoryManager<'a>>>) -> Self {
        let a = Box::leak(value);
        let a = a.as_mut_ptr() as usize;
        DestructuredPhysicalMemory {
            addr: a,
            phantom: PhantomData,
        }
    }
}

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
    #[allow(unused)]
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

impl PciMemory {
    /// Allocate some pci memory with the given size. TODO implement a 32-bit restricted version of this function.
    pub fn new(size: usize) -> Result<Self, core::alloc::AllocError> {
        let mut t = super::super::PAGE_ALLOCATOR.sync_lock();
        let phys = t.extra_mem.allocate_nonram_memory(size, size)?;
        let layout =
            core::alloc::Layout::from_size_align(size, core::mem::size_of::<super::Page>())
                .unwrap();
        let virt = super::super::VIRTUAL_MEMORY_ALLOCATOR.allocate(layout)?;
        let mut mm = super::super::PAGING_MANAGER.sync_lock();
        let va = unsafe { virt.as_ref() }.as_ptr() as usize;
        let pa = unsafe { phys.as_ref() }.as_ptr() as usize;
        match mm.map_addresses_read_write(va, pa, layout.size()) {
            Ok(()) => Ok(unsafe { Self::build_with(va, pa, size) }),
            Err(()) => Err(core::alloc::AllocError),
        }
    }
}

impl<T: Default> DmaMemorySlice<T> {
    /// Construct a new self, with the contents initialized with the default trait
    pub fn new(quantity: usize) -> Result<Self, core::alloc::AllocError> {
        Self::new_with(quantity, |_| Ok(T::default()))
    }
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

    /// Construct a new self, initializing each individual element with a closure
    pub fn new_with(
        quantity: usize,
        mut f: impl FnMut(usize) -> Result<T, core::alloc::AllocError>,
    ) -> Result<Self, core::alloc::AllocError> {
        let mut b: alloc::vec::Vec<T> = alloc::vec::Vec::with_capacity(quantity);
        for i in 0..quantity {
            b.push(f(i)?);
        }
        let va = crate::slice_address(b.as_ref());
        let phys = super::super::PAGING_MANAGER
            .sync_lock()
            .lookup_physical_address(va)
            .ok_or(core::alloc::AllocError)?;
        let s = unsafe { Self::build_with(va, phys, quantity * core::mem::size_of::<T>(), b) };
        Ok(s)
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

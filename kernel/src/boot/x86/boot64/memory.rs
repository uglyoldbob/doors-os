//! This module exists to cover memory management for x64 processors.

use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::{alloc::Allocator, ops::Deref};

use alloc::{boxed::Box, vec::Vec};
use multiboot2::{MemoryAreaType, MemoryMapTag};
use x86_64::registers::control::Cr3Flags;

#[path = "../../memory.rs"]
pub mod generic_memory;

use crate::{address, DestructuredPhysicalMemory, Locked};

extern "C" {
    /// A page table for the system to boot with.
    pub static PAGE_DIRECTORY_BOOT1: PageTable;
    /// The entry for page table level 4
    pub static TABLE4: u64;
    /// The entry for page table level 3
    pub static TABLE3: u64;
    /// The entry for page table level 2
    pub static TABLE2: u64;
    /// The entry for page table level 1
    pub static TABLE1: u64;
}

#[derive(Copy, Clone)]
/// An allocation made by the bump allocator. This is used to undo allocations
struct BumpAllocation {
    /// The size of the allocation in bytes
    bumpsize: usize,
    /// The address of the allocation
    addr: usize,
}

/// A bump allocator for the virtual memory address space of the kernel.
/// It assumes it starts at a given address and own all memory after that point.
pub struct BumpAllocator(Locked<BumpAllocatorInner>);

impl BumpAllocator {
    /// Create a new bump allocator, starting at the specified address
    pub const fn new(addr: usize) -> Self {
        Self(Locked::new(BumpAllocatorInner::new(addr)))
    }

    /// Allocate some memory not backed by ram, normally used for allocating memory for memory mapped devices like pci bar space
    pub fn allocate_nonram_memory(
        &mut self,
        size: usize,
        alignment: usize,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        self.0.sync_lock().allocate_nonram_memory(size, alignment)
    }

    /// Deallocate memory allocated with [allocate_nonram_memory]
    fn deallocate_nonram_memory(
        &mut self,
        ptr: core::ptr::NonNull<u8>,
        layout: core::alloc::Layout,
    ) {
        self.0.sync_lock().deallocate_nonram_memory(ptr, layout);
    }

    /// Relocate the bump allocator to a new address, but only if no addresses are currently out
    pub fn relocate(&mut self, newstart: usize, newend: usize) {
        self.0.sync_lock().relocate(newstart, newend);
    }

    /// Indicates that the bump allocator should start allocating 2mb pages as required
    pub fn start_allocating(&mut self, pt: usize) {
        self.0.sync_lock().start_allocating(pt);
    }

    /// Indicates that the bump allocator should no longer allocate large pages.
    /// It will consider the current end to the end of the current large page to automatically be used.
    pub fn stop_allocating(&mut self, mask: usize) {
        self.0.sync_lock().stop_allocating(mask);
    }
}

/// A bump allocator for the virtual memory address space of the kernel.
/// It assumes it starts at a given address and own all memory after that point.
pub struct BumpAllocatorInner {
    /// The start address for the memory allocation area used by the bump allocator
    start: usize,
    /// The last byte of memory currently allocated by the allocator
    end: usize,
    /// The last few allocations handed out by the bump allocator
    last: [Option<BumpAllocation>; 5],
    /// This option allocates pages of 2mb chunks when set
    allocate_pages: Option<&'static mut PageTable>,
}

impl BumpAllocatorInner {
    /// Create a new bump allocator, starting at the specified address
    pub const fn new(addr: usize) -> Self {
        Self {
            start: addr,
            end: addr,
            last: [None; 5],
            allocate_pages: None,
        }
    }

    /// Allocate some memory not backed by ram, normally used for allocating memory for memory mapped devices like pci bar space
    pub fn allocate_nonram_memory(
        &mut self,
        size: usize,
        alignment: usize,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        let p = self.peek();
        let start = (alignment - 1) & p;
        let waste = if start != 0 { alignment - start } else { 0 };
        if waste != 0 {
            self.waste_space(waste);
        }
        let layout = core::alloc::Layout::from_size_align(size, 1).unwrap();
        self.run_allocation(layout)
    }

    /// Deallocate memory allocated with [allocate_nonram_memory]
    fn deallocate_nonram_memory(
        &mut self,
        ptr: core::ptr::NonNull<u8>,
        layout: core::alloc::Layout,
    ) {
        let layout2 = layout.align_to(layout.size()).unwrap();
        self.run_deallocation(ptr, layout2);
    }

    /// Peek at what the next issued address will start at
    pub fn peek(&mut self) -> usize {
        self.end + 1
    }

    /// Relocate the bump allocator to a new address, but only if no addresses are currently out
    pub fn relocate(&mut self, newstart: usize, newend: usize) {
        if self.start != self.end {
            panic!("Failed to move bump allocator");
        }
        self.start = newstart;
        self.end = newend;
    }

    /// Indicates that the bump allocator should start allocating 2mb pages as required
    pub fn start_allocating(&mut self, pt: usize) {
        self.allocate_pages = Some(unsafe { &mut *(pt as *mut PageTable) });
    }

    /// Indicates that the bump allocator should no longer allocate large pages.
    /// It will consider the current end to the end of the current large page to automatically be used.
    pub fn stop_allocating(&mut self, mask: usize) {
        self.allocate_pages = None;
        let amount = self.end & mask;
        let base = self.end & !mask;
        if amount != 0 {
            self.end = base + mask;
        }
    }

    /// Add a bump allocation to self, returning both the old and new end addresses for this allocator
    fn add_bump_allocation(&mut self, ba: BumpAllocation) -> (usize, usize) {
        for i in 1..5 {
            self.last[i] = self.last[i - 1];
        }
        self.last[0] = Some(ba);
        let old_end = self.end;
        self.end += ba.bumpsize;
        let new_end = self.end;
        (old_end, new_end)
    }

    /// A fake allocation that just wastes space
    pub fn waste_space(&mut self, size: usize) {
        let layout = core::alloc::Layout::from_size_align(size, 1).unwrap();
        let a = BumpAllocation {
            bumpsize: layout.size(),
            addr: self.end + 1,
        };
        self.add_bump_allocation(a);
        self.last[0] = None;
    }

    /// Run an allocation
    pub fn run_allocation(
        &mut self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        let align_mask = layout.align() - 1;
        let align_error = (self.end + 1) & align_mask;
        let align_pad = if align_error > 0 {
            layout.align() - align_error
        } else {
            0
        };
        let bumpsize = layout.size() + align_pad;
        let allocstart = self.end + 1 + align_pad;

        let ptr = unsafe {
            core::ptr::NonNull::new_unchecked(core::ptr::slice_from_raw_parts_mut(
                allocstart as *mut u8,
                layout.size(),
            ))
        };

        let a = BumpAllocation {
            bumpsize,
            addr: allocstart,
        };
        let (old_end, new_end) = self.add_bump_allocation(a);
        if let Some(pa) = &mut self.allocate_pages {
            let mut oldpage = old_end & !0x1fffff;
            let newpage = new_end & !0x1fffff;
            while oldpage != newpage {
                let allpage = oldpage + 0x200000;
                let pageindex = allpage / 0x200000;
                pa.entries[pageindex] = allpage as u64 | 0x83;
                x86_64::instructions::tlb::flush_all();
                oldpage += 0x200000;
            }
        }
        Ok(ptr)
    }

    /// Run a deallocation for the allocator
    fn run_deallocation(&mut self, ptr: core::ptr::NonNull<u8>, _layout: core::alloc::Layout) {
        if let Some(a) = self.last[0] {
            if a.addr == ptr.addr().into() {
                self.end -= a.bumpsize;
                for i in 1..5 {
                    self.last[i - 1] = self.last[i];
                }
                self.last[4] = None;
            }
        }
    }
}

unsafe impl core::alloc::Allocator for Locked<BumpAllocator> {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, alloc::alloc::AllocError> {
        let alloc = self.sync_lock();
        alloc.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        let alloc = self.sync_lock();
        alloc.deallocate(ptr, layout);
    }
}

unsafe impl core::alloc::Allocator for BumpAllocator {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, alloc::alloc::AllocError> {
        let mut alloc = self.0.sync_lock();
        alloc.run_allocation(layout)
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        let mut alloc = self.0.sync_lock();
        alloc.run_deallocation(ptr, layout);
    }
}

unsafe impl core::alloc::Allocator for Locked<BumpAllocatorInner> {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        let mut alloc = self.sync_lock();
        alloc.run_allocation(layout)
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        let mut alloc = self.sync_lock();
        alloc.run_deallocation(ptr, layout);
    }
}

/// A structure for managing which pages are free in a block of contiguous chunks of memory
pub struct Bitmap<'a, T> {
    /// The actual bitmap of available pages, one bit per block
    pub blocks_free: Vec<usize, &'a Locked<BumpAllocator>>,
    /// The start address of the addresses covered by the bitmap
    start: usize,
    /// The number of blocks covered by the bitmap
    num_blocks: usize,
    /// A marker to indicate the struct behaves like it contains a block
    _marker: PhantomData<T>,
}

impl<'a, T> Bitmap<'a, T> {
    /// Create a new bitmap that covers a block of contiguous elements.
    /// # Arguments
    /// * start - The physical address to start allocations
    /// * len - The length of the area to allocate from
    /// * mm - The memory allocator to store the vec of available pages
    fn initialize(start: usize, len: usize, mm: &'a Locked<BumpAllocator>) -> Self {
        let num_pages = len / core::mem::size_of::<T>();
        let num_words = (num_pages + (usize::BITS - 1) as usize) / usize::BITS as usize;

        let mut s = Self {
            blocks_free: Vec::with_capacity_in(num_words, mm),
            start,
            num_blocks: num_pages,
            _marker: PhantomData,
        };

        for _i in 0..num_words {
            s.blocks_free.push(0);
        }

        for i in 0..num_pages {
            let index = i / usize::BITS as usize;
            let offset = i % usize::BITS as usize;
            s.blocks_free[index] |= 1 << offset;
        }

        s
    }

    /// Count the number of free blocks
    fn num_free_blocks(&self) -> usize {
        let mut c = 0;
        for i in 0..self.num_blocks {
            let index = i / usize::BITS as usize;
            let offset = i % usize::BITS as usize;
            let val = self.blocks_free[index] & 1 << offset;
            if val != 0 {
                c += 1;
            }
        }
        c
    }

    /// Used to steal a block of memory from the physical memory manager
    fn steal_block(&mut self, addr: core::ptr::NonNull<u8>) {
        let addr = addr.as_ptr() as usize;
        let start = self.start;
        let i = (addr - start) / core::mem::size_of::<T>();

        let index = i / usize::BITS as usize;
        let offset = i % usize::BITS as usize;
        self.blocks_free[index] &= !(1 << offset);
    }

    /// Return a block to the pool, marking it as available
    fn return_block(&mut self, addr: core::ptr::NonNull<u8>) {
        let addr = addr.as_ptr() as usize;
        let start = self.start;
        let i = (addr - start) / core::mem::size_of::<T>();

        let index = i / usize::BITS as usize;
        let offset = i % usize::BITS as usize;
        self.blocks_free[index] |= 1 << offset;
    }

    /// Retrieve the first available page from the bitmap
    fn get_block(&mut self) -> Option<&mut T> {
        for (index, d) in self.blocks_free.iter_mut().enumerate() {
            if *d != 0 {
                for i in 0..usize::BITS as usize {
                    if (*d & (1 << i)) != 0 {
                        *d &= !(1 << i);
                        let which = index * usize::BITS as usize + i;
                        return Some(unsafe {
                            &mut *((self.start + which * core::mem::size_of::<T>()) as *mut T)
                        });
                    }
                }
            }
        }
        None
    }

    ///Check to see if a page exists in this map
    fn page_exists(&self, d: core::ptr::NonNull<u8>) -> bool {
        let start = self.start;
        let end = start + self.num_blocks * core::mem::size_of::<T>();
        let r = d.as_ptr() as usize;
        r >= start && r < end
    }
}

/// A physical memory page
#[repr(align(4096))]
pub struct Page {
    /// The data for a single physical memory page
    data: [u8; 4096],
}

impl core::ops::Deref for Page {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl core::ops::DerefMut for Page {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl Page {
    /// Create a pattern of alternating bits
    pub fn alternating_bits() -> Self {
        Self { data: [0xaa; 4096] }
    }
}

impl Default for Page {
    fn default() -> Self {
        Self { data: [0; 4096] }
    }
}

#[repr(align(2097152))]
/// A 2 megabyte large page
pub struct Page2Mb {
    /// The page contents
    _data: [Page; 512],
}

/// A simple physical memory manager for the kernel
pub struct SimpleMemoryManager<'a> {
    /// An array of blocks of physical memory managed by the physical memory manager.
    pub bitmaps: Option<Vec<Bitmap<'a, Page>, &'a Locked<BumpAllocator>>>,
    /// The memory manager to get virtual memory, used to allocate space for the bitmaps
    mm: &'a crate::Locked<BumpAllocator>,
    /// The bump allocator for any additional (non-ram) memory for the system, such as pci memory
    extra_mem: BumpAllocator,
}

impl<'a> Locked<SimpleMemoryManager<'a>> {
    /// Do some debugging prints of the memory manager
    pub fn debug(&self) {
        let this = self.sync_lock();
        this.debug();
    }

    /// Maps 4 pages for modification of a page table set
    pub fn modify_four_pages_with_temporary_mapping<
        T,
        F: FnMut([Box<MaybeUninit<PageTable>, &BumpAllocator>; 4]) -> T,
    >(
        &self,
        f: F,
    ) -> T {
        let this = self.sync_lock();
        this.modify_four_pages_with_temporary_mapping(f)
    }

    /// Temporarily assigns a virtual page to a chunk of physical memory, calls a closure with that virtual page, returns a result
    pub fn modify_physical_address_with_temporary_mapping<T, U, F: FnMut(usize, &mut U) -> T>(
        &self,
        addr: usize,
        mut f: F,
    ) -> T {
        let this = self.sync_lock();
        let va = this.mm.sync_lock();
        let mut tvm = Box::<U, &BumpAllocator>::new_uninit_in(va.deref());
        f(addr, unsafe { tvm.assume_init_mut() })
    }

    /// Temporarily assigns a virtual page to a chunk of physical memory, calls a closure with that virtual page, returns a result
    pub fn modify_physical_memory_with_temporary_mapping<
        T,
        U,
        F: FnMut(Box<MaybeUninit<U>, &Locked<SimpleMemoryManager>>, &mut U) -> T,
    >(
        &self,
        phys: Box<MaybeUninit<U>, &Locked<SimpleMemoryManager>>,
        mut f: F,
    ) -> T {
        let this = self.sync_lock();
        let va = this.mm.sync_lock();
        let mut tvm = Box::<U, &BumpAllocator>::new_uninit_in(va.deref());
        f(phys, unsafe { tvm.assume_init_mut() })
    }
}

impl<'a> SimpleMemoryManager<'a> {
    /// Create a new instance of the physical memory manager.
    pub const fn new(mm: &'a crate::Locked<BumpAllocator>) -> Self {
        Self {
            bitmaps: None,
            mm,
            extra_mem: BumpAllocator::new(0x100000),
        }
    }

    /// Maps 4 pages for modification of a page table set
    pub fn modify_four_pages_with_temporary_mapping<
        T,
        F: FnMut([Box<MaybeUninit<PageTable>, &BumpAllocator>; 4]) -> T,
    >(
        &self,
        mut f: F,
    ) -> T {
        let va = self.mm.sync_lock();
        let pages: [Box<MaybeUninit<PageTable>, &BumpAllocator>; 4] = [
            Box::<PageTable, &BumpAllocator>::new_uninit_in(va.deref()),
            Box::<PageTable, &BumpAllocator>::new_uninit_in(va.deref()),
            Box::<PageTable, &BumpAllocator>::new_uninit_in(va.deref()),
            Box::<PageTable, &BumpAllocator>::new_uninit_in(va.deref()),
        ];
        f(pages)
    }

    /// print some debug information about the memory struct
    pub fn debug(&self) {
        if let Some(a1) = &self.bitmaps {
            let a1a = a1.as_ptr();
            crate::VGA.print_str(&alloc::format!("Bitmaps addr is {:p}\r\n", a1a));
            let end = unsafe { &super::super::END_OF_KERNEL } as *const u8 as usize;
            crate::VGA.print_str(&alloc::format!("End of kernel addr is {:x}\r\n", end));
            let mut total_unused_pages = 0;
            for bm in a1 {
                total_unused_pages += bm.num_free_blocks();
                crate::VGA.print_str(&alloc::format!(
                    "block from {:x} size {:x}\r\n",
                    bm.start,
                    bm.num_blocks * 4096
                ));
            }
            crate::VGA.print_str(&alloc::format!(
                "Num free pages is {}\r\n",
                total_unused_pages
            ));
        }
    }

    /// Set a region of memory as used
    pub fn set_area_used(&mut self, start: usize, size: usize) {
        const PAGE_MASK: usize = !(core::mem::size_of::<Page>() - 1);
        if let Some(bitmaps) = &mut self.bitmaps {
            let offset = start & PAGE_MASK;

            let realstart = start - offset;
            let realsize = size + offset;
            let realsize = if (realsize & PAGE_MASK) != 0 {
                (realsize & PAGE_MASK) + core::mem::size_of::<Page>()
            } else {
                realsize
            };
            let realend = realstart + realsize;
            let mut addr = realstart;
            loop {
                for b in bitmaps.iter_mut() {
                    let a = unsafe { core::ptr::NonNull::new_unchecked(addr as *mut u8) };
                    if b.page_exists(a) {
                        b.steal_block(a);
                        break;
                    }
                }
                addr += core::mem::size_of::<Page>();
                if addr >= realend {
                    break;
                }
            }
        }
    }

    /// Assumes memory currently allocated by the bump allocator, as ram currently in use and marks it appropriately
    pub fn set_kernel_memory_used(&mut self) {
        let mml = self.mm.sync_lock();
        let mml = mml.0.sync_lock();

        if let Some(bitmaps) = &mut self.bitmaps {
            for i in (mml.start..mml.end).step_by(core::mem::size_of::<Bitmap<Page>>()) {
                let cadr = unsafe { core::ptr::NonNull::new_unchecked(i as *mut u8) };
                for bitmap in bitmaps.iter_mut() {
                    if bitmap.page_exists(cadr) {
                        bitmap.steal_block(cadr);
                        break;
                    }
                }
            }
        }
    }

    /// Adds a physical memory area to the memory manager. Initializes a new Bitmap in the internal list of bitmaps.
    pub fn add_memory_area(&mut self, ma: &multiboot2::MemoryArea) {
        let mut addr = ma.start_address() as usize;
        let mut size = ma.size() as usize;
        if addr == 0 {
            addr += core::mem::size_of::<Page>();
            size -= core::mem::size_of::<Page>();
        }
        let bm = Bitmap::initialize(addr, size, self.mm);
        if let Some(bitmaps) = &mut self.bitmaps {
            bitmaps.push(bm);
        }
    }

    /// Indicate that there are no more memory areas to add to the memory manager
    pub fn done_adding_memory_areas(&mut self) {
        let mut highest_address: usize = 0;
        for i in self.bitmaps.as_ref().unwrap() {
            let addr: usize = i.start + i.num_blocks * core::mem::size_of::<Page>();
            if addr > highest_address {
                highest_address = addr;
            }
        }
        self.extra_mem.relocate(highest_address, highest_address);
    }

    /// Initialize an instance of a physical memory manager.
    /// This sets up the internal bitmap based on the number of memory segments that are available.
    pub fn init(&mut self, d: &MemoryMapTag) {
        let avail = d
            .memory_areas()
            .iter()
            .filter(|i| i.typ() == MemoryAreaType::Available);
        let n = avail.count();
        let bitmaps: Vec<Bitmap<Page>, &'a Locked<BumpAllocator>> =
            Vec::with_capacity_in(n, self.mm);
        self.bitmaps = Some(bitmaps);
    }

    /// Maps a new page, returning the address of that page. It wil be leaked from the system,
    pub fn get_complete_virtual_page(&mut self) -> usize {
        let a: Box<MaybeUninit<PageTable>, &'a Locked<BumpAllocator>> = Box::new_uninit_in(self.mm);
        Box::<MaybeUninit<PageTable>, &'a Locked<BumpAllocator>>::leak(a)
            as *mut MaybeUninit<PageTable> as usize
    }
}

unsafe impl core::alloc::Allocator for Locked<SimpleMemoryManager<'_>> {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        let mut alloc = self.sync_lock();
        if let Some(bitmaps) = &mut alloc.bitmaps {
            if layout.size() <= core::mem::size_of::<Page>() {
                for bitmap in bitmaps.iter_mut() {
                    if let Some(d) = bitmap.get_block() {
                        return Ok(unsafe {
                            core::ptr::NonNull::new_unchecked(core::ptr::slice_from_raw_parts_mut(
                                d as *mut Page as *mut u8,
                                core::mem::size_of::<Page>(),
                            ))
                        });
                    }
                }
            }
        }
        Err(core::alloc::AllocError)
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, _layout: core::alloc::Layout) {
        let mut alloc = self.sync_lock();
        if let Some(bitmaps) = &mut alloc.bitmaps {
            for bitmap in bitmaps.iter_mut() {
                if bitmap.page_exists(ptr) {
                    bitmap.return_block(ptr);
                    return;
                }
            }
        }
    }
}

impl Drop for generic_memory::PciMemory {
    fn drop(&mut self) {
        let mut t = super::PAGE_ALLOCATOR.sync_lock();
        let layout = core::alloc::Layout::from_size_align(self.size(), self.size()).unwrap();
        t.extra_mem.deallocate_nonram_memory(
            unsafe { core::ptr::NonNull::new_unchecked(self.phys() as *mut u8) },
            layout,
        );
        let layout =
            core::alloc::Layout::from_size_align(self.size(), core::mem::size_of::<Page>())
                .unwrap();
        unsafe {
            super::VIRTUAL_MEMORY_ALLOCATOR.deallocate(
                core::ptr::NonNull::new_unchecked(self.virt() as *mut u8),
                layout,
            )
        };
        let mut mm = super::PAGING_MANAGER.sync_lock();
        mm.unmap_mapped_pages(self.virt(), self.size());
    }
}

impl<T: Default> generic_memory::DmaMemory<T> {
    /// Construct a new self
    pub fn new() -> Result<Self, core::alloc::AllocError> {
        let b: alloc::boxed::Box<T> = alloc::boxed::Box::default();
        let va = crate::address(b.as_ref());
        let phys = super::PAGING_MANAGER
            .sync_lock()
            .lookup_physical_address(va)
            .ok_or(core::alloc::AllocError)?;
        let s = unsafe { Self::build_with(va, phys, core::mem::size_of::<T>(), b) };
        Ok(s)
    }
}

bitfield::bitfield! {
    /// The possible flags for a page table entry
    pub struct PageTableEntryFlags(u16);
    /// Is the item this table refers to present?
    present, set_present: 0;
    /// Is the reference writable?
    writable, set_writable: 1;
    /// Is user access allowed?
    user_access, set_user_access: 2;
    /// Page level write-through
    pwt, set_pwt: 3;
    /// Page level cache disable
    pcd, set_pcd: 4;
    /// Has the page been accessed?
    access, set_access: 5;
    /// Does the entry refer to a larger single chunk of memory?
    huge, set_huge: 7;
}

/// A page table is a part of the paging system. It contains entries that the memory management unit uses to resolve virtual memory addresses to physical memory addresses.
#[repr(align(4096))]
#[repr(C)]
pub struct PageTable {
    /// The array of entries for a page table.
    pub entries: [u64; 512],
}

impl PageTable {
    /// Create a blank page table, all entries set to 0
    const fn new() -> Self {
        Self { entries: [0; 512] }
    }

    /// Sets the entry as present with the specified address, returns the actual address used
    pub fn set_entry(&mut self, index: usize, addr: usize, flags: PageTableEntryFlags) -> u64 {
        self.entries[index] = addr as u64 | flags.0 as u64;
        addr as u64
    }

    /// Returns an address if the entry is marked present
    pub fn get_entry(&self, index: usize) -> Option<u64> {
        let d = self.entries[index];
        if (d & 1) != 0 {
            Some(d & !0xFFF)
        } else {
            None
        }
    }
}

/// Verifies that a PageTable is the correct size
const _PAGETABLE_SPACE_CHECKER: [u8; 4096] = [0; core::mem::size_of::<PageTable>()];

/// The data for a page table modifier
pub struct PageTableModifierData {
    /// The virtual memory address for the affected page
    pub virt: usize,
    /// The virtual memory address of the entry
    pub entry: usize,
}

/// A struct used for modifying mappings of page table
pub struct PageTableModifier<'a> {
    /// The PageTable as it exists in mapped virtual memory
    table: &'a mut PageTable,
    /// The entry used to change the table mapping
    entry: &'a mut usize,
}

impl<'a> PageTableModifier<'a> {
    /// Get the physical address for this page table
    pub fn get_physical_address(&self) -> Option<usize> {
        let d = *self.entry;
        if (d & 1) != 0 {
            Some(d & !0xFFF)
        } else {
            None
        }
    }

    /// Set the physical address for this page table
    pub fn set_physical_address(&mut self, addr: usize) {
        let mut flags = PageTableEntryFlags(0);
        flags.set_present(true);
        flags.set_writable(true);
        *self.entry = addr | flags.0 as usize;
    }

    /// Get the virtual address for this page table
    pub fn get_virtual_address(&self) -> usize {
        crate::address(self.entry)
    }
}

lazy_static::lazy_static! {
    static ref PAGE_TABLE_MAPPER: crate::Locked<[Option<PageTableModifier<'static>>; 4]> = crate::Locked::new([const { None }; 4]);
}

/// Initialize the page table mapper, with a virtual memory allocator. 0 - the address for the usize entry, 1 - the virtual address controlled by the entry
fn init_page_table_mapper(entries: &[PageTableModifierData]) {
    let mut ptm = PAGE_TABLE_MAPPER.sync_lock();
    if ptm[0].is_none() {
        let a = entries[0].virt as *mut PageTable;
        let a = unsafe { a.as_mut() }.unwrap();
        let b = unsafe { (entries[0].entry as *mut usize).as_mut() }.unwrap();
        let p = PageTableModifier { table: a, entry: b };
        ptm[0].replace(p);
    }
    if ptm[1].is_none() {
        let a = entries[1].virt as *mut PageTable;
        let a = unsafe { a.as_mut() }.unwrap();
        let b = unsafe { (entries[1].entry as *mut usize).as_mut() }.unwrap();
        let p = PageTableModifier { table: a, entry: b };
        ptm[1].replace(p);
    }
    if ptm[2].is_none() {
        let a = entries[2].virt as *mut PageTable;
        let a = unsafe { a.as_mut() }.unwrap();
        let b = unsafe { (entries[2].entry as *mut usize).as_mut() }.unwrap();
        let p = PageTableModifier { table: a, entry: b };
        ptm[2].replace(p);
    }
    if ptm[3].is_none() {
        let a = entries[3].virt as *mut PageTable;
        let a = unsafe { a.as_mut() }.unwrap();
        let b = unsafe { (entries[3].entry as *mut usize).as_mut() }.unwrap();
        let p = PageTableModifier { table: a, entry: b };
        ptm[3].replace(p);
    }
}

/// Modifies page tables with a closure
/// # Arguments
/// * cr3 - The cr3 address for the first table
/// * address - The virtual address to modify for
#[inline(never)]
fn modify_page_tables<F: FnMut(usize, &mut PageTable, usize) -> Result<(), ()>>(
    cr3: usize,
    address: usize,
    mut f: F,
) -> Result<(), ()> {
    let mut ptm = PAGE_TABLE_MAPPER.sync_lock();
    let pt4_index = (address >> 39) & 0x1FF;
    let pt3_index = (address >> 30) & 0x1FF;
    let pt2_index = (address >> 21) & 0x1FF;
    let pt1_index = (address >> 12) & 0x1FF;
    let a3 = {
        let pt4 = ptm[0].as_mut().unwrap();
        *pt4.entry = cr3 | 3;
        x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(
            crate::address(pt4.table) as u64
        ));
        f(4, pt4.table, pt4_index)?;
        pt4.table.get_entry(pt4_index).unwrap() as usize
    };
    let a2 = {
        let pt3 = ptm[1].as_mut().unwrap();
        *pt3.entry = a3 | 3;
        x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(
            crate::address(pt3.table) as u64
        ));
        f(3, pt3.table, pt3_index)?;
        pt3.table.get_entry(pt3_index).unwrap() as usize
    };
    let a1 = {
        let pt2 = ptm[2].as_mut().unwrap();
        *pt2.entry = a2 | 3;
        x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(
            crate::address(pt2.table) as u64
        ));
        f(2, pt2.table, pt2_index)?;
        pt2.table.get_entry(pt2_index).unwrap() as usize
    };
    let _a0 = {
        let pt1 = ptm[3].as_mut().unwrap();
        *pt1.entry = a1 | 3;
        x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(
            crate::address(pt1.table) as u64
        ));
        f(1, pt1.table, pt1_index)?;
        pt1.table.get_entry(pt1_index)
    };
    Ok(())
}

/// A manager struct for managing the paging tables for the system. It assumes that a 2mb page is dedicated to viewing page table data.
/// The 4 levels of page tables required for addressing a memory address are loaded as required, changing the mapping in order to
/// modify or examine page tables. If page tables need to be created, then that will be done as required.
pub struct PagingTableManager<'a> {
    /// The physical memory manager reference, used to allocate and deallocate pages used by the paging system.
    mm: &'a crate::Locked<SimpleMemoryManager<'a>>,
    /// The mask for physical addresses
    physical_mask: usize,
    /// The cr3 value for this paging table
    cr3: usize,
}

impl<'a> PagingTableManager<'a> {
    /// Create a new instance of the struct that cannot do anything useful. init must be called at runtime for this object to be useful.
    pub const fn new(mm: &'a crate::Locked<SimpleMemoryManager<'a>>) -> Self {
        Self {
            mm,
            physical_mask: !0,
            cr3: 0,
        }
    }

    /// Install the page as the current page table
    pub unsafe fn install(&self) {
        let (cr3, _) = x86_64::registers::control::Cr3::read();
        let cr3 = cr3.start_address().as_u64() as usize;
        if cr3 != self.cr3 {
            let pa = x86_64::PhysAddr::new_unsafe(self.cr3 as u64);
            let pf = x86_64::structures::paging::PhysFrame::from_start_address_unchecked(pa);
            x86_64::registers::control::Cr3::write(pf, Cr3Flags::PAGE_LEVEL_CACHE_DISABLE);
        }
    }

    /// Copy the kernel map from another paging table into this one.
    /// Other must the the main kernel paging table
    fn copy_kernel_map(&mut self, other: &mut Self) {
        let mut tentry = None;
        modify_page_tables(other.cr3, 0, |level, entry, index| match level {
            4 => {
                tentry = entry.get_entry(index);
                Err(())
            }
            _ => Ok(()),
        });
        modify_page_tables(self.cr3, 0, |level, entry, index| match level {
            4 => {
                let mut flags = PageTableEntryFlags(0);
                flags.set_writable(true);
                flags.set_present(true);
                entry.set_entry(index, tentry.unwrap() as usize, flags);
                Err(())
            }
            _ => Ok(()),
        });
    }

    /// Build a new set of page tables, keeping the existing kernel mappings
    pub fn new_table(&mut self) -> Self {
        let a: Box<MaybeUninit<PageTable>, &Locked<SimpleMemoryManager>> =
            Box::new_uninit_in(self.mm);
        let phys_cr3 = {
            let phys = Box::<PageTable, &Locked<SimpleMemoryManager>>::new_uninit_in(
                &super::PAGE_ALLOCATOR,
            );
            super::PAGE_ALLOCATOR.modify_physical_memory_with_temporary_mapping(phys, |phys, vm| {
                if self
                    .map_addresses_read_write(
                        address(vm),
                        phys.as_ptr() as usize,
                        core::mem::size_of::<PageTable>(),
                    )
                    .is_ok()
                {
                    *vm = PageTable::new();
                    self.unmap_mapped_pages(address(vm), core::mem::size_of::<PageTable>());
                }
                let p: DestructuredPhysicalMemory<PageTable> = phys.into();
                p
            })
        };

        let mut np = Self::new(self.mm);
        np.cr3 = phys_cr3.address();
        np.copy_kernel_map(self);
        np
    }

    /// Lookup the physical address corresponding to the specified address
    fn lookup_physical_address(&mut self, addr: usize) -> Option<usize> {
        let mut paddr = None;
        modify_page_tables(self.cr3, addr, |level, entry, index| match level {
            4 | 3 | 2 => Ok(()),
            1 => {
                paddr = entry.get_entry(index);
                Ok(())
            }
            _ => unreachable!(),
        })
        .ok()?;
        paddr.map(|a| (a as usize) | (addr & 0xFFF))
    }

    /// Set the physical mask according to the number of bits in physical address
    pub fn set_physical_address_size(&mut self, bits: u8) {
        self.physical_mask = (1 << bits) - 1;
    }

    /// Map the virtual address as a window to the given physical address. Used in the init function.
    fn map_window(&mut self, vaddr: usize, phys: u64) -> &'static mut u64 {
        let pml4 = unsafe { &mut *((self.cr3 as u64 & !0xFFF) as *mut PageTable) };
        let pml4_index = (vaddr >> 39) & 0x1FF;
        let pml3 = pml4.get_entry(pml4_index);
        if pml3.is_none() {
            unimplemented!();
        }
        let pml3 = pml3.unwrap();
        let pml3 = unsafe { &mut *(pml3 as *mut PageTable) };

        let pml3_index = (vaddr >> 30) & 0x1FF;
        let pml2 = pml3.get_entry(pml3_index);
        if pml2.is_none() {
            unimplemented!();
        }
        let pml2 = pml2.unwrap();
        let pml2 = unsafe { &mut *(pml2 as *mut PageTable) };

        let pml2_index = (vaddr >> 21) & 0x1FF;
        let mut pml1 = pml2.get_entry(pml2_index);
        if pml1.is_none() {
            let entry: Box<PageTable, &'a crate::Locked<SimpleMemoryManager>> =
                Box::<PageTable, &'a crate::Locked<SimpleMemoryManager>>::new_in(
                    PageTable::new(),
                    self.mm,
                );
            let entry = Box::<PageTable, &'a crate::Locked<SimpleMemoryManager>>::leak(entry);
            pml2.entries[pml2_index] = (entry as *const PageTable as u64) | 1;
            pml1 = pml2.get_entry(pml2_index);
        }
        let pml1 = pml1.unwrap();
        let pml1 = unsafe { &mut *(pml1 as *mut PageTable) };

        let page_table_index = (vaddr >> 12) & 0x1FF;
        pml1.entries[page_table_index] = (phys & !0xFFF) | 1;
        x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(vaddr as u64));
        &mut pml1.entries[page_table_index]
    }

    /// Initialize the object assuming some stuff is already setup in cr3.
    /// entries - 0 - the address for the usize entry, 1 - the virtual address controlled by the entry
    pub fn setup_from_existing(&mut self, entries: &[PageTableModifierData]) {
        let (cr3, _) = x86_64::registers::control::Cr3::read();
        let cr3 = cr3.start_address().as_u64();
        self.cr3 = cr3 as usize;
        init_page_table_mapper(entries);
    }

    /// Map the specified range of physical addresses to the specified virtual addresses as read/write. size is in bytes.
    pub fn map_addresses_read_write(
        &mut self,
        virtual_address: usize,
        physical_address: usize,
        size: usize,
    ) -> Result<(), ()> {
        for i in (0..size).step_by(core::mem::size_of::<Page>()) {
            let vaddr = virtual_address + i;
            let paddr = physical_address + i;
            modify_page_tables(self.cr3, vaddr, |level, entry, index| match level {
                4 => {
                    if entry.get_entry(index).is_none() {
                        let p: Box<MaybeUninit<Page>, &dyn Allocator> = Box::new_uninit_in(self.mm);
                        let p = Box::leak(p);
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        let adr = p.as_ptr() as usize;
                        entry.set_entry(index, adr, flags);
                        x86_64::instructions::tlb::flush_all();
                        *unsafe { p.assume_init_mut() } = Page::default();
                    }
                    Ok(())
                }
                3 => {
                    if entry.get_entry(index).is_none() {
                        let p: Box<MaybeUninit<Page>, &dyn Allocator> = Box::new_uninit_in(self.mm);
                        let p = Box::leak(p);
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        entry.set_entry(index, p.as_ptr() as usize, flags);
                        x86_64::instructions::tlb::flush_all();
                        *unsafe { p.assume_init_mut() } = Page::default();
                    }
                    Ok(())
                }
                2 => {
                    if entry.get_entry(index).is_none() {
                        let p: Box<MaybeUninit<Page>, &dyn Allocator> = Box::new_uninit_in(self.mm);
                        let p = Box::leak(p);
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        entry.set_entry(index, p.as_ptr() as usize, flags);
                        x86_64::instructions::tlb::flush_all();
                        *unsafe { p.assume_init_mut() } = Page::default();
                    }
                    Ok(())
                }
                1 => {
                    if entry.get_entry(index).is_none() {
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        entry.set_entry(index, paddr, flags);
                        x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(vaddr as u64));
                    } else {
                        return Err(());
                    }
                    Ok(())
                }
                _ => unreachable!(),
            })?;
        }
        Ok(())
    }

    /// Map the specified range of physical addresses to the specified virtual addresses. size corresponds to bytes
    pub fn map_addresses_read_only(
        &mut self,
        virtual_address: usize,
        physical_address: usize,
        size: usize,
    ) -> Result<(), ()> {
        for i in (0..size).step_by(core::mem::size_of::<Page>()) {
            let vaddr = virtual_address + i;
            let paddr = physical_address + i;
            modify_page_tables(self.cr3, vaddr, |level, entry, index| match level {
                4 => {
                    if entry.get_entry(index).is_none() {
                        let p: Box<MaybeUninit<Page>, &dyn Allocator> = Box::new_uninit_in(self.mm);
                        let p = Box::leak(p);
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        entry.set_entry(index, p.as_ptr() as usize, flags);
                        x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(
                            crate::address(entry) as u64,
                        ));
                        *unsafe { p.assume_init_mut() } = Page::default();
                    }
                    Ok(())
                }
                3 => {
                    if entry.get_entry(index).is_none() {
                        let p: Box<MaybeUninit<Page>, &dyn Allocator> = Box::new_uninit_in(self.mm);
                        let p = Box::leak(p);
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        entry.set_entry(index, p.as_ptr() as usize, flags);
                        x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(
                            crate::address(entry) as u64,
                        ));
                        *unsafe { p.assume_init_mut() } = Page::default();
                    }
                    Ok(())
                }
                2 => {
                    if entry.get_entry(index).is_none() {
                        let p: Box<MaybeUninit<Page>, &dyn Allocator> = Box::new_uninit_in(self.mm);
                        let p = Box::leak(p);
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        entry.set_entry(index, p.as_ptr() as usize, flags);
                        x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(
                            crate::address(entry) as u64,
                        ));
                        *unsafe { p.assume_init_mut() } = Page::default();
                    }
                    Ok(())
                }
                1 => {
                    if entry.get_entry(index).is_none() {
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(false);
                        flags.set_present(true);
                        entry.set_entry(index, paddr, flags);
                        x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(vaddr as u64));
                    }
                    Ok(())
                }
                _ => unreachable!(),
            })?;
        }
        Ok(())
    }

    /// Unmaps some pages that were previously mapped, size is in bytes
    pub fn unmap_mapped_pages(&mut self, virtual_address: usize, size: usize) {
        for i in (0..size).step_by(core::mem::size_of::<Page>()).rev() {
            let vaddr = virtual_address + i;
            modify_page_tables(self.cr3, vaddr, |level, entry, index| match level {
                4 => {
                    if entry.get_entry(index).is_none() {
                        let p: Box<MaybeUninit<Page>, &dyn Allocator> = Box::new_uninit_in(self.mm);
                        let p = Box::leak(p);
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        entry.set_entry(index, p.as_ptr() as usize, flags);
                        x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(
                            crate::address(entry) as u64,
                        ));
                        *unsafe { p.assume_init_mut() } = Page::default();
                    }
                    Ok(())
                }
                3 => {
                    if entry.get_entry(index).is_none() {
                        let p: Box<MaybeUninit<Page>, &dyn Allocator> = Box::new_uninit_in(self.mm);
                        let p = Box::leak(p);
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        entry.set_entry(index, p.as_ptr() as usize, flags);
                        x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(
                            crate::address(entry) as u64,
                        ));
                        *unsafe { p.assume_init_mut() } = Page::default();
                    }
                    Ok(())
                }
                2 => {
                    if entry.get_entry(index).is_none() {
                        let p: Box<MaybeUninit<Page>, &dyn Allocator> = Box::new_uninit_in(self.mm);
                        let p = Box::leak(p);
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        entry.set_entry(index, p.as_ptr() as usize, flags);
                        x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(
                            crate::address(entry) as u64,
                        ));
                        *unsafe { p.assume_init_mut() } = Page::default();
                    }
                    Ok(())
                }
                1 => {
                    let mut flags = PageTableEntryFlags(0);
                    entry.set_entry(index, 0, flags);
                    x86_64::instructions::tlb::flush(x86_64::addr::VirtAddr::new(vaddr as u64));
                    Ok(())
                }
                _ => unreachable!(),
            });
        }
    }

    /// Unmap a mapped page and deallocate the physical page that is mapped to it.
    pub fn unmap_delete_page(&mut self, address: usize) -> Result<(), ()> {
        loop {}
        Ok(())
    }

    /// Map a virtual memory address to a page which will be grabbed from the physical memory manager.
    pub fn map_new_page(&mut self, address: usize) -> Result<(), ()> {
        let physical_page: Box<MaybeUninit<Page>, &'a crate::Locked<SimpleMemoryManager>> =
            Box::new_uninit_in(self.mm);
        let physical_page = unsafe { physical_page.assume_init() };
        let paddr = Box::leak(physical_page);
        let paddr = crate::address(paddr);
        self.map_addresses_read_write(address, paddr, core::mem::size_of::<Page>())
    }
}

/// Responsible for mapping physical memory to virtual memory
pub struct VirtualMemoryMapper<'a> {
    ptm: &'a Locked<PagingTableManager<'a>>,
}

impl<'a> VirtualMemoryMapper<'a> {
    /// Build a new mapper
    pub const fn new(ptm: &'a Locked<PagingTableManager<'a>>) -> Self {
        Self { ptm }
    }
}

impl<'a> crate::boot::VirtualMemoryMapperTrait for VirtualMemoryMapper<'a> {
    fn allocate_physical_pages(&self, virt: usize, len: usize) -> Result<(), ()> {
        let mut ptm = self.ptm.sync_lock();
        for i in (0..len).step_by(core::mem::size_of::<Page>()) {
            let vaddr = virt + i;
            ptm.map_new_page(vaddr)?;
        }
        Ok(())
    }

    fn unallocate_physical_pages(&self, virt: usize, len: usize) -> Result<(), ()> {
        let mut ptm = self.ptm.sync_lock();
        for i in (0..len).step_by(core::mem::size_of::<Page>()) {
            let vaddr = virt + i;
            ptm.unmap_delete_page(vaddr)?;
        }
        Ok(())
    }
}

//! This module exists to cover memory management for x86 (32 bit) processors. It assumes the usage of physical address extensions.

use core::marker::PhantomData;
use core::mem::MaybeUninit;

use alloc::{alloc::Allocator, boxed::Box, vec::Vec};
use multiboot2::MemoryMapTag;

#[path = "../../memory.rs"]
pub mod memory;

use crate::Locked;

/// The page directory, used for the paging system in PAGE paging.
pub static PAGE_DIRECTORY_BOOT1: PageTable = PageTable { entries: [0; 512] };

extern "C" {
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
    #[allow(unused)]
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

    /// Add a bumpallocation to self, returning both the old and new end addresses for this allocator
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
                unsafe { x86::tlb::flush_all() };
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
#[allow(unused)]
pub struct Page {
    /// The data for a single physical memory page
    data: [u8; 4096],
}

impl Page {
    /// Create a blank page, filled with zeros
    pub fn new() -> Self {
        Self { data: [0; 4096] }
    }

    /// Get a raw pointer to Self
    pub fn as_ptr(&self) -> *const Self {
        self as *const Self
    }
}

impl Default for Page {
    fn default() -> Self {
        Self { data: [0; 4096] }
    }
}

#[repr(align(0x400000))]
/// A 4 megabyte large page
#[allow(unused)]
pub struct Page4Mb {
    /// The page contents
    data: [Page; 1024],
}

/// A 4 megabyte large page
pub struct Page4MbMapped {
    /// The page address
    address: usize,
}

impl Page4MbMapped {
    /// Create a mapping to a 4mb page containing the specified address
    pub fn from_raw(start: usize) -> Self {
        let cr3 = unsafe { x86::controlregs::cr3() };
        let table_addr = cr3 & 0xFFFFF000;
        let t: &mut [u32] =
            unsafe { core::slice::from_raw_parts_mut(table_addr as *mut u32, 1024) };
        let page_addr = start & 0xFFC00000;
        let page_index = start >> 22;
        t[page_index] = (page_addr | 0x83) as u32;
        unsafe { x86::tlb::flush_all() };
        Self { address: page_addr }
    }
}

impl Drop for Page4MbMapped {
    fn drop(&mut self) {
        let cr3 = unsafe { x86::controlregs::cr3() };
        let table_addr = cr3 & 0xFFFFF000;
        let t: &mut [u32] =
            unsafe { core::slice::from_raw_parts_mut(table_addr as *mut u32, 1024) };
        let start = self.address;
        let page_addr = start & 0xFFC00000;
        let page_index = start >> 22;
        t[page_index] = (page_addr | 0x83) as u32;
        unsafe { x86::tlb::flush_all() };
    }
}

/// A simple physical memory manager for the kernel
pub struct SimpleMemoryManager<'a> {
    /// An array of blocks of physical memory managed by the physical memory manager.
    pub bitmaps: Option<Vec<Bitmap<'a, Page>, &'a Locked<BumpAllocator>>>,
    /// The memory manager to get virtual memory, used to allocate space for the bitmaps
    mm: &'a crate::Locked<BumpAllocator>,
    /// The bump allocator for any additional memory for the system
    extra_mem: BumpAllocator,
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

    /// Adds a memory area to the memory manager
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

    /// Initialize an instance of a physical memory manager
    pub fn init(&mut self, d: &MemoryMapTag) {
        let avail = d
            .memory_areas()
            .iter()
            .filter(|i| i.typ() == multiboot2::MemoryAreaType::Available);
        let n = avail.count();
        let bitmaps: Vec<Bitmap<Page>, &'a Locked<BumpAllocator>> =
            Vec::with_capacity_in(n, self.mm);
        self.bitmaps = Some(bitmaps);
    }

    /// Maps a new page, returning the address of that page. It wil be l3eaked from the system,
    pub fn get_complete_virtual_page(&mut self) -> usize {
        let a: Box<MaybeUninit<PageTable>, &'a Locked<BumpAllocator>> = Box::new_uninit_in(self.mm);
        Box::<MaybeUninit<PageTable>, &'a Locked<BumpAllocator>>::leak(a)
            as *mut MaybeUninit<PageTable> as usize
    }
}

unsafe impl<'a> core::alloc::Allocator for Locked<SimpleMemoryManager<'a>> {
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
    pub fn set_entry(&mut self, index: usize, addr: u64, flags: PageTableEntryFlags) -> u64 {
        self.entries[index] = addr | flags.0 as u64;
        addr
    }

    /// Returns an address if the entry is marked present
    fn get_entry(&self, index: usize) -> Option<u64> {
        let d = self.entries[index];
        if (d & 1) != 0 {
            Some(d & !0xFFF)
        } else {
            None
        }
    }
}

/// A reference to a page table, used for the windowing scheme. A page table is mapped into virtual memory and points to a physical page.
/// This struct keeps track of the window of virtual memory used to examine a page table physically located at physical_address.
/// This is because the x86 paging scheme uses physical addresses in its page tables instead of virtual addresses.
struct PageTableRef {
    ///A reference to the page table
    table: &'static mut PageTable,
    /// The entry in a page table that allows the mapping to change
    virtual_mapping: &'static mut u32,
}

impl PageTableRef {
    /// Create a blank page table ref, using the specified address for viewing a page table.
    #[allow(unused)]
    const fn blank(a: &mut PageTable, v: &'static mut u32) -> Self {
        Self {
            table: unsafe { &mut *(a as *mut PageTable) },
            virtual_mapping: v,
        }
    }

    /// Create a page table ref, fully specified with virtual address and page table entry reference.
    fn new(virt: usize, v: &'static mut u32) -> Self {
        Self {
            table: unsafe { (virt as *mut PageTable).as_mut().unwrap() },
            virtual_mapping: v,
        }
    }

    /// Get the address of the page table viewing window
    #[allow(unused)]
    fn table_address(&self) -> usize {
        self.table as *const PageTable as usize
    }

    /// Update the current page table reference to the given physical address if required, return true if any action was required.
    fn update(&mut self, phys: u32) -> bool {
        if phys != *self.virtual_mapping {
            *self.virtual_mapping = phys | 1;
            unsafe { x86::tlb::flush(self.table as *const PageTable as usize) };
            true
        } else {
            false
        }
    }
}

/// A manager struct for managing the paging tables for the system. For PAE paging, there are several layers of tables.
/// The page tables required for addressing a memory address are loaded as required, changing the mapping in order to
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
        let cr3 = unsafe { x86::controlregs::cr3() } as usize;
        if cr3 != self.cr3 {
            x86::controlregs::cr3_write(self.cr3 as u64);
        }
    }

    /// Initialize the object assuming some stuff is already setup in cr3.
    /// entries - 0 - the address for the usize entry, 1 - the virtual address controlled by the entry
    pub fn setup_from_existing(&mut self, entries: &[PageTableModifierData]) {
        self.cr3 = unsafe { x86::controlregs::cr3() } as usize;
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
                3 => {
                    if entry.get_entry(index).is_none() {
                        let p: Box<MaybeUninit<Page>, &dyn Allocator> = Box::new_uninit_in(self.mm);
                        let p = Box::leak(p);
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        entry.set_entry(index, p.as_ptr() as u64, flags);
                        unsafe { x86::tlb::flush_all() };
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
                        entry.set_entry(index, p.as_ptr() as u64, flags);
                        unsafe { x86::tlb::flush_all() };
                        *unsafe { p.assume_init_mut() } = Page::default();
                    }
                    Ok(())
                }
                1 => {
                    if entry.get_entry(index).is_none() {
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        entry.set_entry(index, paddr as u64, flags);
                        unsafe { x86::tlb::flush(vaddr)};
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

    /// Lookup the physical address corresponding to the specified address
    fn lookup_physical_address(&mut self, addr: usize) -> Option<usize> {
        let mut paddr = None;
        modify_page_tables(self.cr3, addr, |level, entry, index| match level {
            3 | 2 => Ok(()),
            1 => {
                paddr = entry.get_entry(index);
                Ok(())
            }
            _ => unreachable!(),
        })
        .ok()?;
        paddr.map(|a| (a as usize) | (addr & 0xFFF))
    }

    /// Map the virtual address as a window to the given physical address. Used in the init function.
    fn map_window(&mut self, vaddr: usize, phys: u64) -> &'static mut u64 {
        let cr3 = unsafe { x86::controlregs::cr3() } as usize;
        let page_directory: &mut PageTable =
            unsafe { &mut *((cr3 & 0xFFFFF000) as *mut PageTable) };
        let mut page_table = page_directory.entries[(vaddr >> 22) & 0x3FF];
        if (page_table & 1) == 0 {
            let page_directory_entry: Box<PageTable, &'a crate::Locked<SimpleMemoryManager>> =
                Box::<PageTable, &'a crate::Locked<SimpleMemoryManager>>::new_in(
                    PageTable::new(),
                    self.mm,
                );
            let page_directory_entry =
                Box::<PageTable, &'a crate::Locked<SimpleMemoryManager>>::leak(
                    page_directory_entry,
                );
            page_directory.entries[(vaddr >> 22) & 0x3FF] =
                (page_directory_entry as *const PageTable as u64) | 1;
            page_table = page_directory.entries[(vaddr >> 22) & 0xFF];
        }
        let page_directory_entry = unsafe {
            ((page_table & 0xFFFFF000) as *mut PageTable)
                .as_mut()
                .unwrap()
        };
        let page_table_index = (vaddr >> 12) & 0x3FF;
        page_directory_entry.entries[page_table_index] = (phys & 0xFFFFF000) | 1;
        unsafe { x86::tlb::flush(vaddr) };
        &mut page_directory_entry.entries[page_table_index]
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
                3 => {
                    if entry.get_entry(index).is_none() {
                        let p: Box<MaybeUninit<Page>, &dyn Allocator> = Box::new_uninit_in(self.mm);
                        let p = Box::leak(p);
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        entry.set_entry(index, p.as_ptr() as u64, flags);
                        unsafe { x86::tlb::flush(crate::address(entry)) };
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
                        entry.set_entry(index, p.as_ptr() as u64, flags);
                        unsafe { x86::tlb::flush(crate::address(entry)) };
                        *unsafe { p.assume_init_mut() } = Page::default();
                    }
                    Ok(())
                }
                1 => {
                    if entry.get_entry(index).is_none() {
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(false);
                        flags.set_present(true);
                        entry.set_entry(index, paddr as u64, flags);
                        unsafe { x86::tlb::flush(vaddr) };
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
                3 => {
                    if entry.get_entry(index).is_none() {
                        let p: Box<MaybeUninit<Page>, &dyn Allocator> = Box::new_uninit_in(self.mm);
                        let p = Box::leak(p);
                        let mut flags = PageTableEntryFlags(0);
                        flags.set_writable(true);
                        flags.set_present(true);
                        entry.set_entry(index, p.as_ptr() as u64, flags);
                        unsafe { x86::tlb::flush(crate::address(entry)) };
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
                        entry.set_entry(index, p.as_ptr() as u64, flags);
                        unsafe { x86::tlb::flush(crate::address(entry)) };
                        *unsafe { p.assume_init_mut() } = Page::default();
                    }
                    Ok(())
                }
                1 => {
                    let mut flags = PageTableEntryFlags(0);
                    entry.set_entry(index, 0, flags);
                    unsafe { x86::tlb::flush(vaddr) };
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

/// The data for a page table modifier
pub struct PageTableModifierData {
    /// The virtual memory address for the affected page
    pub virt: usize,
    /// The virtual memory address of the entry
    pub entry: usize,
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

/// A struct used for modifying mappings of page table
pub struct PageTableModifier<'a> {
    /// The PageTable as it exists in mapped virtual memory
    table: &'a mut PageTable,
    /// The entry used to change the table mapping
    entry: &'a mut u64,
}

impl<'a> PageTableModifier<'a> {
    /// Get the physical address for this page table
    pub fn get_physical_address(&self) -> Option<u64> {
        let d = *self.entry;
        if (d & 1) != 0 {
            Some(d & !0xFFF)
        } else {
            None
        }
    }

    /// Set the physical address for this page table
    pub fn set_physical_address(&mut self, addr: u64) {
        let mut flags = PageTableEntryFlags(0);
        flags.set_present(true);
        flags.set_writable(true);
        *self.entry = addr | flags.0 as u64;
    }

    /// Get the virtual address for this page table
    pub fn get_virtual_address(&self) -> usize {
        crate::address(self.entry)
    }
}

lazy_static::lazy_static! {
    static ref PAGE_TABLE_MAPPER: crate::Locked<[Option<PageTableModifier<'static>>; 3]> = crate::Locked::new([const { None }; 3]);
}

/// Initialize the page table mapper, with a virtual memory allocator. 0 - the address for the usize entry, 1 - the virtual address controlled by the entry
fn init_page_table_mapper(entries: &[PageTableModifierData]) {
    let mut ptm = PAGE_TABLE_MAPPER.sync_lock();
    if ptm[0].is_none() {
        let a = entries[0].virt as *mut PageTable;
        let a = unsafe { a.as_mut() }.unwrap();
        let b = unsafe { (entries[0].entry as *mut u64).as_mut() }.unwrap();
        let p = PageTableModifier { table: a, entry: b };
        ptm[0].replace(p);
    }
    if ptm[1].is_none() {
        let a = entries[1].virt as *mut PageTable;
        let a = unsafe { a.as_mut() }.unwrap();
        let b = unsafe { (entries[1].entry as *mut u64).as_mut() }.unwrap();
        let p = PageTableModifier { table: a, entry: b };
        ptm[1].replace(p);
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
    let pt3_index = (address >> 30) & 0x3;
    let pt2_index = (address >> 21) & 0x1FF;
    let pt1_index = (address >> 12) & 0x1FF;
    let a2 = {
        let pt3 = ptm[0].as_mut().unwrap();
        *pt3.entry = cr3 as u64 | 3;
        unsafe { x86::tlb::flush(crate::address(pt3.table)) };
        f(3, pt3.table, pt3_index)?;
        pt3.table.get_entry(pt3_index).unwrap()
    };
    let a1 = {
        let pt2 = ptm[1].as_mut().unwrap();
        *pt2.entry = a2 | 3;
        unsafe { x86::tlb::flush(crate::address(pt2.table)) };
        f(2, pt2.table, pt2_index)?;
        pt2.table.get_entry(pt2_index).unwrap()
    };
    let _a0 = {
        let pt1 = ptm[2].as_mut().unwrap();
        *pt1.entry = a1 | 3;
        unsafe { x86::tlb::flush(crate::address(pt1.table)) };
        f(1, pt1.table, pt1_index)?;
        pt1.table.get_entry(pt1_index)
    };
    Ok(())
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

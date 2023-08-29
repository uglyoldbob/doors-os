//! This module exists to cover memory management for x86 (32 bit) processors. It assumes the usage of physical address extensions.

use core::marker::PhantomData;
use core::mem::MaybeUninit;

use alloc::{boxed::Box, vec::Vec};
use doors_kernel_api::video::TextDisplay;
use multiboot2::MemoryMapTag;

use crate::Locked;

use crate::x86::VGA;
use doors_kernel_api::FixedString;

/// The page directory pointer table, used for the paging system in PAE paging.
pub static mut PAGE_DIRECTORY_POINTER_TABLE: PageDirectoryPointerTable =
    PageDirectoryPointerTable::new();

/// The page directory, used for the paging system in PAGE paging.
pub static mut PAGE_DIRECTORY_BOOT1: PageTable = PageTable { entries: [0; 512] };

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
pub struct BumpAllocator {
    /// The start address for the memory allocation area used by the bump allocator
    start: usize,
    /// The last byte of memory currently allocated by the allocator
    end: usize,
    /// The last few allocations handed out by the bump allocator
    last: [Option<BumpAllocation>; 5],
    /// This option allocates pages of 2mb chunks when set
    allocate_pages: Option<&'static mut PageTable>,
}

impl BumpAllocator {
    /// Create a new bump allocator, starting at the specified address
    pub const fn new(addr: usize) -> Self {
        Self {
            start: addr,
            end: addr,
            last: [None; 5],
            allocate_pages: None,
        }
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
}

unsafe impl core::alloc::Allocator for Locked<BumpAllocator> {
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        let mut alloc = self.lock();
        let align_mask = layout.align() - 1;
        let align_error = (alloc.end + 1) & align_mask;
        let align_pad = if align_error > 0 {
            layout.align() - align_error
        } else {
            0
        };
        let bumpsize = layout.size() + align_pad;
        let allocstart = alloc.end + 1 + align_pad;

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
        for i in 1..5 {
            alloc.last[i] = alloc.last[i - 1];
        }
        alloc.last[0] = Some(a);
        let old_end = alloc.end;
        alloc.end += bumpsize;
        let new_end = alloc.end;
        if let Some(pa) = &mut alloc.allocate_pages {
            let mut oldpage = old_end & !(core::mem::size_of::<Page2Mb>() - 1);
            let newpage = new_end & !(core::mem::size_of::<Page2Mb>() - 1);
            while oldpage != newpage {
                let allpage = oldpage + core::mem::size_of::<Page2Mb>();
                let pageindex = allpage / core::mem::size_of::<Page2Mb>();
                pa.entries[pageindex] = allpage as u64 | 0x83;
                unsafe { x86::tlb::flush_all() };
                oldpage += core::mem::size_of::<Page2Mb>();
            }
        }
        Ok(ptr)
    }

    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, _layout: core::alloc::Layout) {
        let mut alloc = self.lock();
        if let Some(a) = alloc.last[0] {
            if a.addr == ptr.addr().into() {
                alloc.end -= a.bumpsize;
                for i in 1..5 {
                    alloc.last[i - 1] = alloc.last[i];
                }
                alloc.last[4] = None;
            }
        }
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

#[repr(align(2097152))]
/// A 2 megabyte large page
pub struct Page2Mb {
    /// The page contents
    data: [Page; 512],
}

/// A simple physical memory manager for the kernel
pub struct SimpleMemoryManager<'a> {
    /// An array of blocks of physical memory managed by the physical memory manager.
    pub bitmaps: Option<Vec<Bitmap<'a, Page>, &'a Locked<BumpAllocator>>>,
    /// The memory manager to get virtual memory, used to allocate space for the bitmaps
    mm: &'a crate::Locked<BumpAllocator>,
}

impl<'a> SimpleMemoryManager<'a> {
    /// Create a new instance of the physical memory manager.
    pub const fn new(mm: &'a crate::Locked<BumpAllocator>) -> Self {
        Self { bitmaps: None, mm }
    }

    /// Assumes memory currently allocated by the bump allocator, as ram currently in use and marks it appropriately
    pub fn set_kernel_memory_used(&mut self) {
        let mml = self.mm.lock();

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
        let mut alloc = self.lock();
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
        let mut alloc = self.lock();
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

/// This is the structure that cr3 points to
#[repr(align(32))]
#[repr(C)]
pub struct PageDirectoryPointerTable {
    /// Each of these 4 entries refers to a 1gb chunk of memory
    /// This entry points to a page directory pointer and is 32-bits long, the upper 32 bits are reserved.
    /// The bottom 5 bits are ignored
    entries: [u64; 4],
}

impl PageDirectoryPointerTable {
    /// Create a new blank table.
    pub const fn new() -> Self {
        Self { entries: [0; 4] }
    }

    /// Get a raw pointer to self.
    pub fn get_ptr(&self) -> usize {
        self as *const Self as usize
    }

    /// Get the pagetable that corresponds to the given virtual address
    pub fn get_pagetable(&mut self, vaddr: u32) -> Option<&mut PageTable> {
        let index = vaddr >> 30;
        let d = self.entries[index as usize];
        if (d & 1) != 0 {
            unsafe { ((d & 0xFFFFF000) as *mut PageTable).as_mut() }
        }
        else {
            None
        }
    }

    /// Get the entry that corresponds to the given virtual address
    pub fn get_entry(&mut self, vaddr: u32) -> Option<u64> {
        let index = vaddr >> 30;
        let d = self.entries[index as usize];
        if (d & 1) != 0 {
            Some(d & !0xFFF)
        }
        else {
            None
        }
    }

    /// Set the entry for the given virtual address with the specified pageTable
    pub fn set_pagetable(&mut self, vaddr: u32, pt: &PageTable) {
        let index = vaddr >> 30;
        self.entries[index as usize] = pt as *const PageTable as u64 | 1;
    }

    /// Assign this object to cr3
    pub fn assign_to_cr3(&self) {
        unsafe {
            x86::controlregs::cr3_write(self.get_ptr() as u64);
            x86::tlb::flush_all();
        }
    }
}

/// A reference to an existing page directory pointer table
pub struct PageDirectoryPointerTableRef {
    /// The physical address
    physical_address: usize,
    /// The table in virtual memory
    table: &'static mut PageDirectoryPointerTable,
}

impl PageDirectoryPointerTableRef {
    /// Create a new object with the specified cr3 and virtual address
    fn new(cr3: usize, virt: usize) -> Self {
        let virtaddr = virt | (cr3 & 0xFE0);
        let virtaddr = virtaddr as *mut PageDirectoryPointerTable;
        doors_macros2::kernel_print!("Virtual address is {:p}\r\n", virtaddr);
        Self {
            physical_address: cr3,
            table: unsafe { virtaddr.as_mut().unwrap() },
        }
    }

    /// Update this reference as necessary to point to the specified cr3, return true if the function did anything
    fn update(&mut self, cr3: usize) -> bool {
        let mywindow = self.physical_address & 0xFFFFF000;
        let window = cr3 & 0xFFFFF000;
        if mywindow != window {
            self.physical_address = window;
            let virt = (self.table as *mut PageDirectoryPointerTable as usize) & 0xFFFFE00;
            let virt = virt | (cr3 & 0xFE0);
            let virtaddr = virt as *mut PageDirectoryPointerTable;
            self.table = unsafe { virtaddr.as_mut().unwrap() };
            true
        }
        else {
            false
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

    /// Returns an address if the entry is marked present
    fn get_entry(&mut self, index: usize) -> Option<u64> {
        let d = self.entries[index];
        if (d & 1) != 0 {
            Some(d & !0xFFF)
        }
        else {
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
    virtual_mapping: &'static mut u64,
}

impl PageTableRef {
    /// Create a blank page table ref, using the specified address for viewing a page table.
    const fn blank(a: &mut PageTable, v: &'static mut u64) -> Self {
        Self {
            table: unsafe { &mut *(a as *mut PageTable) },
            virtual_mapping: v,
        }
    }

    /// Create a page table ref, fully specified with virtual address and page table entry reference.
    fn new(virt: usize, v: &'static mut u64) -> Self {
        Self {
            table: unsafe { (virt as *mut PageTable).as_mut().unwrap() },
            virtual_mapping: v,
        }
    }

    /// Get the address of the page table viewing window
    fn table_address(&self) -> usize {
        self.table as *const PageTable as usize
    }

    /// Update the current page table reference to the given physical address if required, return true if any action was required.
    fn update(&mut self, phys: u64) -> bool {
        if phys != *self.virtual_mapping {
            *self.virtual_mapping = phys | 1;
            unsafe { x86::tlb::flush(self.table as *const PageTable as usize) };
            true
        }
        else {
            false
        }
    }
}

/// A manager struct for managing the paging tables for the system. For PAE paging, there are several layers of tables.
/// The page tables required for addressing a memory address are loaded as required, changing the mapping in order to
/// modify or examine page tables. If page tables need to be created, then that will be done as required.
///
/// The top level page table is a set of 4 64-bit entries (not a full page), each covering a 1 GB address space. Each entry points to a page directory.
/// The second level is a page directory. Each 64-bit entry points to a page table or to a 2MB page. The entire page directory maps 1GB of memory.
/// The third level is a page table, each entry either points to a 4kb page. The full page table maps 2MB.
/// A page table is required in order to bring other page table into view for the paging table manager.
pub struct PagingTableManager<'a> {
    /// For the cr3 value
    pdpt: MaybeUninit<PageDirectoryPointerTableRef>,
    /// For the second level page table.
    pt2: MaybeUninit<PageTableRef>,
    /// For the first level page table.
    pt1: MaybeUninit<PageTableRef>,
    /// The physical memory manager reference, used to allocate and deallocate pages used by the paging system.
    mm: &'a crate::Locked<SimpleMemoryManager<'a>>,
}

impl<'a> PagingTableManager<'a> {
    /// Create a new instance of the struct that cannot do anything useful. init must be called at runtime for this object to be useful.
    pub const fn new(mm: &'a crate::Locked<SimpleMemoryManager<'a>>) -> Self {
        Self {
            pdpt: MaybeUninit::uninit(),
            pt2: MaybeUninit::uninit(),
            pt1: MaybeUninit::uninit(),
            mm,
        }
    }

    /// Map the virtual address as a window to the given physical address. Used in the init function.
    fn map_window(&mut self, vaddr: usize, phys: u64) -> &'static mut u64 {
        let cr3 = unsafe { x86::controlregs::cr3() } as usize;
        let pdpt_temp = (cr3 & 0xFFFFFFE0) as *mut PageDirectoryPointerTable;
        let pdpt_temp = unsafe { pdpt_temp.as_mut().unwrap() };
        let mut page_directory = pdpt_temp.get_pagetable(vaddr as u32);
        if page_directory.is_none() {
            unimplemented!();
        }
        let mut page_directory = page_directory.unwrap();
        let mut page_table = page_directory.entries[(vaddr >> 21) & 0x1FF];

        if (page_table & 1) == 0 {
            let mut page_directory_entry: Box<PageTable, &'a crate::Locked<SimpleMemoryManager>> =
                Box::<PageTable, &'a crate::Locked<SimpleMemoryManager>>::new_in(
                    PageTable::new(),
                    self.mm,
                );
            let page_directory_entry = Box::<PageTable, &'a crate::Locked<SimpleMemoryManager>>::leak(page_directory_entry);
            page_directory.entries[(vaddr >> 21) & 0x1FF] =
                (page_directory_entry as *const PageTable as u64) | 1;
            page_table = page_directory.entries[(vaddr >> 21) & 0x1FF];
        }
        let page_directory_entry = unsafe {
            ((page_table & 0xFFFFFE00) as *mut PageTable)
                .as_mut()
                .unwrap()
        };
        let page_table_index = (vaddr >> 12) & 0x1FF;
        page_directory_entry.entries[page_table_index] = (phys & 0xFFFFFFE0) | 1;
        unsafe { x86::tlb::flush(vaddr) };
        &mut page_directory_entry.entries[page_table_index]
    }

    /// Initialize the object, allocating physical pages as required.
    pub fn init(&mut self) {
        let cr3 = unsafe { x86::controlregs::cr3() } as usize;

        let mut mm = self.mm.lock();
        let pdpt_window = mm.get_complete_virtual_page();
        let page_directory_window = mm.get_complete_virtual_page();
        let page_table_window = mm.get_complete_virtual_page();
        drop(mm);

        let a = self.map_window(pdpt_window, cr3 as u64);
        let b = self.map_window(page_directory_window, 0);
        let c = self.map_window(page_table_window, 0);

        let pdpt = PageDirectoryPointerTableRef::new(cr3, pdpt_window);
        self.pdpt = MaybeUninit::new(pdpt);
        self.pt2 = MaybeUninit::new(PageTableRef::new(page_directory_window, b));
        self.pt1 = MaybeUninit::new(PageTableRef::new(page_table_window, c));
    }

    /// Setup the page table pointers with the given cr3 and address value so that page tables can be examined or modified.
    fn setup_cache(&mut self, cr3: usize, address: usize) {
        doors_macros2::kernel_print!("Setting up cache for paging {:x}\r\n", address);
        unsafe { &mut *self.pdpt.as_mut_ptr()}.update(cr3);
        let mut gigabyte = unsafe {&mut *self.pdpt.as_mut_ptr() }.table.get_entry(address as u32);
        if gigabyte.is_none() {
            unimplemented!();
            gigabyte = unsafe {&mut *self.pdpt.as_mut_ptr() }.table.get_entry(address as u32);
        }
        let gigabyte = gigabyte.unwrap();
        unsafe { &mut *self.pt2.as_mut_ptr()}.update(gigabyte);

        let mut page_directory = unsafe {&mut *self.pdpt.as_mut_ptr() }.table.get_pagetable(address as u32).unwrap();

        let pde = page_directory.get_entry((address >> 21) & 0x1FF);
        let pde = match pde {
            Some(pde) => {
                pde
            }
            None => {
                unimplemented!();
                0
            }
        };
        unsafe { &mut *self.pt1.as_mut_ptr()}.update(pde);
    }

    /// Map the specified range of physical addresses to the specified virtual addresses. size corresponds to bytes
    pub fn map_addresses_read_only(
        &mut self,
        virtual_address: usize,
        physical_address: usize,
        size: usize,
    ) -> Result<(), ()> {
        let cr3 = unsafe { x86::controlregs::cr3() } as usize;

        for i in (0..size).step_by(core::mem::size_of::<Page>()) {
            let vaddr = virtual_address + i;
            let paddr = physical_address + i;
            self.setup_cache(cr3, vaddr);
            let pt1_index = ((vaddr >> 12) & 0x1FF) as usize;

            if (unsafe { &*self.pt1.as_ptr() }.table.entries[pt1_index] & 1) == 0 {
                unsafe { &mut *self.pt1.as_mut_ptr() }.table.entries[pt1_index] =
                    paddr as u64 | 0x1;
                unsafe { x86::tlb::flush(vaddr) };
            } else {
                return Err(());
            }

            if size > 0x5000 && i > 63000 * core::mem::size_of::<Page>() as usize {
                loop {}
            }
        }
        Ok(())
    }

    /// Unmaps some pages that were previously mapped, size is in bytes
    pub fn unmap_mapped_pages(&mut self, virtual_address: usize, size: usize) {
        let cr3 = unsafe { x86::controlregs::cr3() } as usize;

        for i in (0..size).step_by(core::mem::size_of::<Page>()) {
            let vaddr = virtual_address + i;
            self.setup_cache(cr3, vaddr);
            let pt1_index = ((vaddr >> 12) & 0x1FF) as usize;
            if (unsafe { &*self.pt1.as_ptr() }.table.entries[pt1_index] & 1) != 0 {
                unsafe { &mut *self.pt1.as_mut_ptr() }.table.entries[pt1_index] = 0;
                unsafe { x86::tlb::flush(vaddr) };
            }
        }
    }

    /// Unmap a mapped page and deallocate the physical page that is mapped to it.
    pub fn unmap_delete_page(&mut self, address: usize) -> Result<(), ()> {
        let cr3 = unsafe { x86::controlregs::cr3() } as usize;

        self.setup_cache(cr3, address);

        let pt1_index = ((address >> 12) & 0x1FF) as usize;

        if (unsafe { &*self.pt1.as_ptr() }.table.entries[pt1_index] & 1) != 0 {
            let a = unsafe { &*self.pt1.as_ptr() }.table.entries[pt1_index] & 0xFFFFF000;
            let addr = a as *mut PageTable;
            let entry: Box<PageTable, &'a crate::Locked<SimpleMemoryManager>> =
                unsafe { Box::from_raw_in(addr, self.mm) };
            drop(entry);
            unsafe { &mut *self.pt1.as_mut_ptr() }.table.entries[pt1_index] = 0;
            //TODO determine if pt1 is empty
            unsafe { x86::tlb::flush(address) };
            Ok(())
        } else {
            Err(())
        }
    }

    /// Map a memory address to a page which will be grabbed from the physical memory manager.
    pub fn map_new_page(&mut self, address: usize) -> Result<(), ()> {
        let cr3 = unsafe { x86::controlregs::cr3() } as usize;
        self.setup_cache(cr3, address);
        doors_macros2::kernel_print!("Mapping new page to {:x}\r\n", address);
        let pt1_index = ((address >> 12) & 0x1FF) as usize;

        if (unsafe { &*self.pt1.as_ptr() }.table.entries[pt1_index] & 1) == 0 {
            let entry: Box<PageTable, &'a crate::Locked<SimpleMemoryManager>> =
                Box::new_in(PageTable::new(), self.mm);
            let entry: &mut PageTable =
                Box::<PageTable, &'a crate::Locked<SimpleMemoryManager>>::leak(entry);
            let addr = entry as *const PageTable as usize;
            unsafe { &mut *self.pt1.as_mut_ptr() }.table.entries[pt1_index] = addr as u64 | 0x3;
            unsafe { x86::tlb::flush(address) };
            Ok(())
        } else {
            Err(())
        }
    }
}

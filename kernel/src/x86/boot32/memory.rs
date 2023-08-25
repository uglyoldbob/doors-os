//! This module exists to cover memory management for x86 (32 bit) processors. It assumes the usage of physical address extensions.

use core::marker::PhantomData;

use alloc::{boxed::Box, vec::Vec};
use doors_kernel_api::video::TextDisplay;
use multiboot2::MemoryMapTag;

use crate::Locked;

use crate::x86::VGA;
use doors_kernel_api::FixedString;

pub static mut PAGE_DIRECTORY_POINTER_TABLE: PageDirectoryPointerTable =
    PageDirectoryPointerTable::new();

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

    /// Indicates that the bump allocator should no longer allocate 2mb pages
    pub fn stop_allocating(&mut self) {
        self.allocate_pages = None;
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
    fn new() -> Self {
        Self { data: [0; 4096] }
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
#[repr(align(4096))]
#[repr(C)]
pub struct PageDirectoryPointerTable {
    /// Each of these 4 entries refers to a 1gb chunk of memory
    /// This entry points to a page directory pointer and is 32-bits long, the upper 32 bits are reserved.
    /// The bottom 5 bits are ignored
    entries: [u64; 4],
}

impl PageDirectoryPointerTable {
    pub const fn new() -> Self {
        Self { entries: [0; 4] }
    }

    pub fn get_ptr(&self) -> usize {
        self as *const Self as usize
    }

    pub fn get_entry(&self, paddr: u32) -> &mut PageTable {
        let index = paddr >> 29;
        unsafe { ((self.entries[index as usize] & 0xFFFFF000) as *mut PageTable).as_mut() }.unwrap()
    }

    pub fn set_entry(&mut self, paddr: u32, pt: &PageTable) {
        let index = paddr >> 29;
        self.entries[index as usize] = pt as *const PageTable as u64 | 1;
    }

    pub fn assign_to_cr3(&self) {
        unsafe {
            x86::controlregs::cr3_write(self.get_ptr() as u64);
            x86::tlb::flush_all();
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
}

/// A reference to a page table, used for the windowing scheme. A page table is mapped into virtual memory and points to a physical page.
/// This struct keeps track of the window of virtual memory used to examine a page table physically located at physical_address.
/// This is because the x86 paging scheme uses physical addresses in its page tables instead of virtual addresses.
struct PageTableRef {
    ///A reference to the page table
    table: &'static mut PageTable,
    ///The physical address of the table
    physical_address: usize,
}

impl PageTableRef {
    /// Create a blank page table ref, using the specified address for viewing a page table.
    const fn blank(a: usize) -> Self {
        Self {
            table: unsafe { &mut *(a as *mut PageTable) },
            physical_address: 0,
        }
    }

    /// Get the address of the page table viewing window
    fn table_address(&self) -> usize {
        self.table as *const PageTable as usize
    }

    /// Set the physical address of the page table
    fn set_address(&mut self, d: usize) {
        self.physical_address = d | 1;
    }

    /// Return the physical address, if it is valid
    fn address(&self) -> Option<usize> {
        if (self.physical_address & 1) != 0 {
            Some(self.physical_address)
        } else {
            None
        }
    }
}

/// A manager struct for managing the paging tables for the system. It assumes that a 2mb page is dedicated to viewing page table data.
/// The 4 levels of page tables required for addressing a memory address are loaded as required, changing the mapping in order to
/// modify or examine page tables. If page tables need to be created, then that will be done as required.
pub struct PagingTableManager<'a> {
    /// This is the page that corresponds to the 2mb section for viewing page table data.
    page2mb: Option<&'a mut PageTable>,
    /// For the second level page table.
    pt2: Option<PageTableRef>,
    /// For the first level page table.
    pt1: Option<PageTableRef>,
    /// The physical memory manager reference, used to allocate and deallocate pages used by the paging system.
    mm: &'a crate::Locked<SimpleMemoryManager<'a>>,
}

impl<'a> PagingTableManager<'a> {
    /// Create a new instance of the struct that cannot do anything useful. init must be called at runtime for this object to be useful.
    pub const fn new(mm: &'a crate::Locked<SimpleMemoryManager<'a>>) -> Self {
        Self {
            page2mb: None,
            pt2: None,
            pt1: None,
            mm,
        }
    }

    /// Initialize the object, using the address mp as the starting address for the 2 megabyte page used for managing the page tables of the system.
    pub fn init(&mut self, mp: usize) {
        if (mp & 0x1FFFFF) != 0 {
            super::super::VGA
                .lock()
                .print_str("Invalid memory location for paging window");
            loop {}
        }

        let cr3 = unsafe { x86::controlregs::cr3() } as usize;

        doors_macros2::kernel_print!("Cr3 is {:x}\r\n", cr3);

        let pdpt = unsafe { &mut *(cr3 as *mut PageDirectoryPointerTable) };
        let pt2t = pdpt.get_entry(mp as u32);
        let pt2_index = (mp >> 21) & 0x1ff;
        doors_macros2::kernel_print!("pdpt is {:p}\r\n", pdpt);
        doors_macros2::kernel_print!("pt2t is {:p}\r\n", pt2t);
        doors_macros2::kernel_print!("pt2ti is {:x}\r\n", pt2_index);
        let pt1 = pt2t.entries[pt2_index as usize];
        doors_macros2::kernel_print!("pt1 is {:x}\r\n", pt1);

        if (pt1 & 1) != 0 {
            super::super::VGA
                .lock()
                .print_str("Memory for paging already occupied");
        }

        let new_2mb_entry: Box<PageTable, &'a Locked<SimpleMemoryManager>> =
            Box::new_in(PageTable::new(), self.mm);
        let new_2mb_entry = Box::<PageTable, &Locked<SimpleMemoryManager>>::leak(new_2mb_entry);
        let addr = new_2mb_entry as *const PageTable as usize;
        doors_macros2::kernel_print!("Setup page map from {:x} to {:x} {}\r\n", addr, mp, pt2_index);
        pt2t.entries[pt2_index as usize] = addr as u64 | 0x3;
        unsafe { x86::tlb::flush(mp) };
        doors_macros2::kernel_print!("cache setup: {:x}\r\n", mp);
        self.page2mb = Some(new_2mb_entry);
        self.pt2 = Some(PageTableRef::blank(mp + 1 * 0x1000));
        self.pt1 = Some(PageTableRef::blank(mp + 2 * 0x1000));
    }

    /// Setup the page table pointers with the given cr3 and address value so that page tables can be examined or modified.
    fn setup_cache(&mut self, cr3: usize, address: usize) {
        super::super::VGA
                .lock()
                .print_str("Setting up cache for paging\r\n");
        let pt2_index = ((address >> 21) & 0x1FF) as usize;


        let mut pt2addr: usize = 0;
        let mut pt1addr: usize = 0;

        if let Some(page2mb) = &mut self.page2mb {
            pt2addr = page2mb.entries[address >> 30] as usize;
            if let Some(pt2) = &mut self.pt2 {
                if pt2addr != (pt2.physical_address & 0xFFFFF000) {
                    page2mb.entries[3] = pt2addr as u64 | 0x3;
                    doors_macros2::kernel_print!("pt2 address is {:x}\r\n", pt2addr);
                    pt2.set_address(pt2addr);
                    unsafe { x86::tlb::flush(pt2.table_address() as usize) };
                }

                doors_macros2::kernel_print!("Map {:p}\r\n", &pt2.table.entries);
                loop {}

                if (pt2.table.entries[pt2_index] & 1) == 0 {
                    let entry: Box<PageTable, &'a crate::Locked<SimpleMemoryManager>> =
                        Box::new_in(PageTable::new(), self.mm);
                    let entry: &mut PageTable =
                        Box::<PageTable, &'a crate::Locked<SimpleMemoryManager>>::leak(entry);
                    let addr = entry as *const PageTable as usize;
                    pt2.table.entries[pt2_index] = addr as u64 | 0x3;
                }

                pt1addr = pt2.table.entries[pt2_index] as usize & 0xFFFFF000;
            }

            if let Some(pt1) = &mut self.pt1 {
                if pt1addr != (pt1.physical_address & 0xFFFFF000) {
                    page2mb.entries[4] = pt1addr as u64 | 0x3;
                    pt1.set_address(pt1addr);
                    unsafe { x86::tlb::flush(pt1.table_address() as usize) };
                }
            }
        }
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

            if let Some(pt1) = &mut self.pt1 {
                if (pt1.table.entries[pt1_index] & 1) == 0 {
                    pt1.table.entries[pt1_index] = paddr as u64 | 0x1;
                    unsafe { x86::tlb::flush(vaddr) };
                } else {
                    return Err(());
                }
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
            if let Some(pt1) = &mut self.pt1 {
                if (pt1.table.entries[pt1_index] & 1) != 0 {
                    pt1.table.entries[pt1_index] = 0;
                    unsafe { x86::tlb::flush(vaddr) };
                }
            }
        }
    }

    /// Unmap a mapped page and deallocate the physical page that is mapped to it.
    pub fn unmap_delete_page(&mut self, address: usize) -> Result<(), ()> {
        let cr3 = unsafe { x86::controlregs::cr3() } as usize;

        self.setup_cache(cr3, address);

        let pt1_index = ((address >> 12) & 0x1FF) as usize;

        if let Some(pt1) = &mut self.pt1 {
            if (pt1.table.entries[pt1_index] & 1) != 0 {
                let a = pt1.table.entries[pt1_index] & 0xFFFFF000;
                let addr = a as *mut PageTable;
                let entry: Box<PageTable, &'a crate::Locked<SimpleMemoryManager>> =
                    unsafe { Box::from_raw_in(addr, self.mm) };
                drop(entry);
                pt1.table.entries[pt1_index] = 0;
                //TODO determine if pt1 is empty
                unsafe { x86::tlb::flush(address) };
                Ok(())
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    /// Map a memory address to a page which will be grabbed from the physical memory manager.
    pub fn map_new_page(&mut self, address: usize) -> Result<(), ()> {
        let cr3 = unsafe { x86::controlregs::cr3() } as usize;
        self.setup_cache(cr3, address);
        doors_macros2::kernel_print!("Mapping new page to {:x}\r\n", address);
        loop {}
        let pt1_index = ((address >> 12) & 0x1FF) as usize;

        if let Some(pt1) = &mut self.pt1 {
            if (pt1.table.entries[pt1_index] & 1) == 0 {
                let entry: Box<PageTable, &'a crate::Locked<SimpleMemoryManager>> =
                    Box::new_in(PageTable::new(), self.mm);
                let entry: &mut PageTable =
                    Box::<PageTable, &'a crate::Locked<SimpleMemoryManager>>::leak(entry);
                let addr = entry as *const PageTable as usize;
                pt1.table.entries[pt1_index] = addr as u64 | 0x3;
                unsafe { x86::tlb::flush(address) };
                Ok(())
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }
}

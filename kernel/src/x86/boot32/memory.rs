//! This module exists to cover memory management for x86 (32 bit) processors. It assumes the usage of physical address extensions.

use core::marker::PhantomData;

use alloc::{boxed::Box, vec::Vec};
use doors_kernel_api::video::TextDisplay;
use multiboot2::MemoryMapTag;

use crate::Locked;

use crate::x86::VGA;
use doors_kernel_api::FixedString;

pub static mut PAGE_DIRECTORY_POINTER_TABLE: PageDirectoryPointerTable = PageDirectoryPointerTable::new();

pub static mut PAGE_DIRECTORY_BOOT1: PageTable = PageTable {
    entries: [0; 512],
};

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
        doors_macros2::kernel_print!("allocator: {:p}, {:x} {:x}\r\n", self, alloc.start, alloc.end);
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
        doors_macros2::kernel_print!("ptr is {:p}\r\n", unsafe { ptr.as_ref() });
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
        let avail: Vec<&multiboot2::MemoryArea> = d.memory_areas().iter().filter(|i| i.typ() == multiboot2::MemoryAreaType::Available).collect();
        let n = avail.len();
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
        Self {
            entries: [0;4],
        }
    }

    pub fn get_ptr(&self) -> usize {
        self as *const Self as usize
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
    page4mb: Option<&'a mut PageTable>,
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
            page4mb: None,
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

        let pt2t = unsafe { &mut *(cr3 as *mut PageTable) };
        let pt2_index = (mp >> 21) & 0x1FF;
        let pt1 = pt2t.entries[pt2_index as usize];

        if (pt1 & 1) != 0 {
            super::super::VGA
                .lock()
                .print_str("Memory for paging already occupied");
            loop {}
        }

        let new_4mb_entry: Box<PageTable, &'a Locked<SimpleMemoryManager>> =
            Box::new_in(PageTable::new(), self.mm);
        let new_4mb_entry = Box::<PageTable, &Locked<SimpleMemoryManager>>::leak(new_4mb_entry);
        let addr = new_4mb_entry as *const PageTable as usize;
        pt2t.entries[pt2_index as usize] = addr as u64 | 0x3;
        unsafe { x86::tlb::flush(mp) };

        self.page4mb = Some(new_4mb_entry);
        self.pt2 = Some(PageTableRef::blank(mp + 1 * 0x1000));
        self.pt1 = Some(PageTableRef::blank(mp + 2 * 0x1000));
    }

    /// Setup the page table pointers with the given cr3 and address value so that page tables can be examined or modified.
    fn setup_cache(&mut self, cr3: usize, address: usize) {
        let pt2_index = ((address >> 21) & 0x1FF) as usize;

        let mut pt2addr : usize = 0;
        let mut pt1addr : usize = 0;

        if let Some(page4mb) = &mut self.page4mb {
            if let Some(pt2) = &mut self.pt2 {
                if pt2addr != (pt2.physical_address & 0xFFFFF000) {
                    page4mb.entries[3] = pt2addr as u64 | 0x3;
                    pt2.set_address(pt2addr);
                    unsafe { x86::tlb::flush(pt2.table_address() as usize) };
                }

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
                    page4mb.entries[4] = pt1addr as u64 | 0x3;
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

/// A container structure for a heap node
/// Stores some calculations about how the node can allocate a chunk of memory
struct HeapNodeAlign {
    /// The total size needed to fulfill the desired memory allocation
    size_needed: usize,
    /// The number of bytes of padding that occur between the start of the node and the start of the allocation
    pre_align: usize,
    /// The number of bytes at the end of the allocation to meet alignment for node size
    post_align: usize,
}

#[derive(Debug)]
/// A node of free memory for the heap
struct HeapNode<'a> {
    /// The optional next node of free memory for the heap
    next: Option<*mut HeapNode<'a>>,
    /// The size of this node, including the size of this header
    size: usize,
}

impl<'a> HeapNode<'a> {
    /// The required alignment for nodes and allocations based on the size of a node
    const NODEALIGN: usize = core::mem::size_of::<HeapNode>().next_power_of_two();

    /// Return the address that is immediately after this block of memory
    fn next_address(&self) -> usize {
        self as *const HeapNode as usize + self.size
    }

    /// Return the start address of this free block
    fn start(&self) -> usize {
        self as *const HeapNode as usize
    }

    /// Create a heap node, at the specified location, using the specified layout
    unsafe fn with_ptr(ptr: *mut u8, layout: core::alloc::Layout) -> *mut Self {
        let node = ptr as *mut Self;
        let size = layout.size();
        let err = size % Self::NODEALIGN;
        let s = if err != 0 {
            size + Self::NODEALIGN - err
        } else {
            size
        };
        (*node).size = s;
        (*node).next = None;
        node
    }

    /// Calculate the alignment properties of an allocation for this node.
    /// This fits a chunk of memory of size bytes and align alignment.
    fn calc_alignment(&self, size: usize, align: usize) -> HeapNodeAlign {
        let align_mask = align - 1;
        let align_err = self.start() & align_mask;
        let align_pad = if align_err != 0 { align - align_err } else { 0 };
        let size_needed = align_pad + size;
        let posterr = (self.start() + align_pad + size) % Self::NODEALIGN;
        let postpad = if posterr != 0 {
            Self::NODEALIGN - posterr
        } else {
            0
        };
        HeapNodeAlign {
            size_needed: size_needed + postpad,
            pre_align: align_pad,
            post_align: posterr,
        }
    }
}

/// The heap manager for the system. It assumes it starts at a given address and expands to the end of known memory.
pub struct HeapManager<'a> {
    /// The beginning of the list of free memory nodes.
    head: Option<*mut HeapNode<'a>>,
    /// The paging table manager, used to map additional memory into the heap as required.
    mm: &'a crate::Locked<PagingTableManager<'a>>,
    /// The allocator for getting more virtual memory
    vmm: &'a crate::Locked<BumpAllocator>,
}

unsafe impl<'a> Send for HeapManager<'a> {}

impl<'a> HeapManager<'a> {
    /// Create a heap manager.
    pub const fn new(
        mm: &'a crate::Locked<PagingTableManager<'a>>,
        vmm: &'a crate::Locked<BumpAllocator>,
    ) -> Self {
        Self {
            head: None,
            mm,
            vmm,
        }
    }

    /// Print details of the heap
    fn print(&self) {
        if let Some(mut r) = self.head {
            loop {
                let addr = unsafe { &(*r) }.start();

                let mut tp: doors_kernel_api::FixedString = doors_kernel_api::FixedString::new();
                match core::fmt::write(
                    &mut tp,
                    format_args!("heap node is {:?} {:x}\r\n", unsafe { &*r }, addr),
                ) {
                    Ok(_) => super::super::VGA.lock().print_str(tp.as_str()),
                    Err(_) => super::super::VGA
                        .lock()
                        .print_str("Error parsing string\r\n"),
                }

                if let Some(nr) = unsafe { &(*r) }.next {
                    r = nr;
                } else {
                    break;
                }
            }
        } else {
            super::super::VGA.lock().print_str("Heap is empty\r\n");
        }
    }

    /// Expand the heap by a certain amount, using real memory.
    fn expand_with_physical_memory(&mut self, amount: usize) -> Result<(), ()> {
        // Round up a partial page to a whole page, a is number of pages, not number of bytes
        let (a, r) = (
            amount / core::mem::size_of::<Page>(),
            amount % core::mem::size_of::<Page>(),
        );
        let a = if r != 0 { a + 1 } else { a };

        let new_section = Vec::<Page, &Locked<BumpAllocator>>::with_capacity_in(a, self.vmm);

        let sa = new_section.as_ptr() as *const Page as usize;
        let mut mm = self.mm.lock();
        for i in (sa..sa + a * core::mem::size_of::<Page>()).step_by(core::mem::size_of::<Page>()) {
            mm.map_new_page(i as usize)?;
        }
        drop(mm);

        if self.head.is_none() {
            let node = new_section.as_ptr() as *mut HeapNode;
            unsafe { (*node).next = None };
            unsafe { (*node).size = new_section.capacity() * core::mem::size_of::<Page>() };
            self.head = Some(node);
        } else {
            self.print();
            unimplemented!();
        }
        new_section.leak();
        Ok(())
    }

    /// Perform an actual allocation
    fn run_alloc(&mut self, layout: core::alloc::Layout) -> *mut u8 {
        if self.head.is_none() {
            if let Err(_) = self.expand_with_physical_memory(layout.size() + layout.align()) {
                return core::ptr::null_mut();
            }
        }

        let mut elem = self.head;
        let mut prev_elem: Option<*mut HeapNode> = None;
        let mut best_fit_link: &mut Option<*mut HeapNode> = &mut None;
        let mut best_fit: Option<*mut HeapNode> = None;
        let mut best_fit_ha: Option<HeapNodeAlign> = None;
        while let Some(h) = elem {
            let ha = unsafe { (*h).calc_alignment(layout.size(), layout.align()) };
            if ha.size_needed <= unsafe { (*h).size } {
                if let Some(b) = best_fit {
                    if unsafe { (*h).size } < unsafe { (*b).size } {
                        best_fit_link = if let Some(pe) = prev_elem {
                            unsafe { &mut (*pe).next }
                        } else {
                            &mut self.head
                        };
                        best_fit = elem;
                        best_fit_ha = Some(ha);
                    }
                } else {
                    best_fit_link = if let Some(pe) = prev_elem {
                        unsafe { &mut (*pe).next }
                    } else {
                        &mut self.head
                    };
                    best_fit = elem;
                    best_fit_ha = Some(ha);
                }
            }
            prev_elem = elem;
            elem = unsafe { (*h).next };
        }

        if let Some(best) = best_fit {
            let ha = best_fit_ha.unwrap();
            let r = if ha.pre_align < core::mem::size_of::<HeapNode>() {
                if (unsafe { (*best).size } - ha.size_needed) < core::mem::size_of::<HeapNode>() {
                    super::super::VGA
                        .lock()
                        .print_str("The entire block will be used\r\n");
                    self.print();
                    unimplemented!();
                } else {
                    let after_node = unsafe { (*best).start() } + ha.size_needed;
                    let node = after_node as *mut HeapNode;
                    unsafe { (*node).size = (*best).size - ha.size_needed };
                    *best_fit_link = Some(node);
                    (unsafe { (*best).start() } + ha.pre_align) as *mut u8
                }
            } else {
                super::super::VGA
                    .lock()
                    .print_str("A free node will exist before the placement\r\n");
                if (unsafe { (*best).size } - ha.size_needed) < core::mem::size_of::<HeapNode>() {
                    super::super::VGA
                        .lock()
                        .print_str("The end of the block will be used\r\n");
                } else {
                    super::super::VGA
                        .lock()
                        .print_str("There will be blank space at the end of the block\r\n");
                }
                self.print();
                unimplemented!();
            };
            r
        } else {
            super::super::VGA
                .lock()
                .print_str("Heap node not found?\r\n");
            core::ptr::null_mut()
        }
    }

    /// Perform an actual deallocation
    fn run_dealloc(&mut self, ptr: *mut u8, layout: core::alloc::Layout) {
        let new_node = unsafe { HeapNode::with_ptr(ptr, layout) };
        let e = self.head.take();
        unsafe { (*new_node).next = e };
        self.head = Some(new_node);

        //TODO merge blocks if possible?
    }
}

unsafe impl<'a> core::alloc::GlobalAlloc for Locked<HeapManager<'a>> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut alloc = self.lock();
        let layout2 = layout.align_to(HeapNode::NODEALIGN).unwrap();
        alloc.run_alloc(layout2)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let mut alloc = self.lock();
        let layout2 = layout.align_to(HeapNode::NODEALIGN).unwrap();
        alloc.run_dealloc(ptr, layout2);
    }
}

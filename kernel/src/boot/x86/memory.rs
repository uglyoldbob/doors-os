//! Memory management code that is common to both 32 and 64 bit x86.
//! The heap is a linked list of memory areas ([HeapNode]).
//! Memory is organized by blocks that represent free memory.

use alloc::vec::Vec;
use core::ptr::NonNull;

use crate::modules::video::TextDisplayTrait;
use crate::Locked;

use super::boot::memory::{Page, PagingTableManager};

pub use super::boot::memory::BumpAllocator as Allocator;

/// A container structure for a heap node
/// Stores some calculations about how the node can allocate a chunk of memory
struct HeapNodeAlign {
    /// The total size needed to fulfill the desired memory allocation
    size_needed: usize,
    /// The number of bytes of padding that occur between the start of the node and the start of the allocation
    pre_align: usize,
    /// The number of bytes at the end of the allocation to meet alignment for node size
    _post_align: usize,
}

#[derive(Debug)]
/// A node of memory for the heap
struct HeapNode {
    /// The optional next node of memory for the heap
    next: Option<NonNull<HeapNode>>,
    /// The size of this node, including the size of this header
    size: usize,
}

impl HeapNode {
    /// The required alignment for nodes and allocations based on the size of a node
    const NODEALIGN: usize = core::mem::size_of::<HeapNode>();

    fn print(&self) {
        doors_macros2::kernel_print!("heap node is {:p} {:?}\r\n", self, self);
    }

    fn next(&self) -> Option<&Self> {
        self.next.map(|a| unsafe { a.as_ref() })
    }

    /// Return the start address of this free block
    fn start(&self) -> usize {
        self as *const HeapNode as usize
    }

    /// Attempt to merge this block with the ones that come after it
    fn try_merge_with_next(&mut self) {
        while let Some(n) = self.next {
            if (crate::address(self) + self.size) == (n.as_ptr() as usize) {
                let nn = unsafe { n.as_ref() }.next;
                let nsize = unsafe { n.as_ref() }.size;
                self.next = nn;
                self.size += nsize;
            } else {
                break;
            }
        }
    }

    /// Create a heap node, at the specified location, using the specified layout
    unsafe fn with_ptr(ptr: *mut u8, layout: core::alloc::Layout) -> NonNull<Self> {
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
        NonNull::<Self>::new_unchecked(node)
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
        if align_pad != 0 {
            doors_macros2::kernel_print!(
                "ALIGN PAD CALCULATED {:x} {:x} {:x}\r\n",
                self.start(),
                size,
                align
            );
            doors_macros2::kernel_print!("\talign_mask: {:x}\r\n", align_mask);
            doors_macros2::kernel_print!("\talign_err: {:x}\r\n", align_err);
            doors_macros2::kernel_print!("\talign_pad: {:x}\r\n", align_pad);
            doors_macros2::kernel_print!("\tsize_needed: {:x}\r\n", size_needed);
            doors_macros2::kernel_print!("\tposterr: {:x}\r\n", posterr);
            doors_macros2::kernel_print!("\tpostpad: {:x}\r\n", postpad);
        }
        HeapNodeAlign {
            size_needed: size_needed + postpad,
            pre_align: align_pad,
            _post_align: posterr,
        }
    }
}

/// The heap manager for the system. It assumes it starts at a given address and expands to the end of known memory.
pub struct HeapManager<'a> {
    /// The beginning of the list of free memory nodes.
    head: Option<NonNull<HeapNode>>,
    /// The paging table manager, used to map additional memory into the heap as required.
    mm: &'a crate::Locked<PagingTableManager<'a>>,
    /// The allocator for getting more virtual memory
    vmm: &'a crate::Locked<Allocator>,
}

unsafe impl Send for HeapManager<'_> {}

impl<'a> HeapManager<'a> {
    /// Create a heap manager.
    pub const fn new(
        mm: &'a crate::Locked<PagingTableManager<'a>>,
        vmm: &'a crate::Locked<Allocator>,
    ) -> Self {
        Self {
            head: None,
            mm,
            vmm,
        }
    }

    /// Print details of the heap
    pub fn print(&self) {
        doors_macros2::kernel_print!("mm: {:p}\r\n", self.mm);
        doors_macros2::kernel_print!("vmm: {:p}\r\n", self.vmm);
        if let Some(mut r) = self.head {
            let mut t = unsafe { r.as_ref() };
            doors_macros2::kernel_print!("HEAP:\r\n");
            t.print();
            loop {
                match t.next() {
                    Some(a) => {
                        a.print();
                        t = a;
                    }
                    None => {
                        break;
                    }
                }
            }
            while let Some(t2) = t.next() {
                t2.print();
            }
        } else {
            doors_macros2::kernel_print!("Heap is empty\r\n");
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

        let new_section = Vec::<Page, &Locked<Allocator>>::with_capacity_in(a, self.vmm);

        let sa = new_section.as_ptr() as usize;
        let mut mm = self.mm.lock();
        for i in (sa..sa + a * core::mem::size_of::<Page>()).step_by(core::mem::size_of::<Page>()) {
            mm.map_new_page(i)?;
        }
        drop(mm);

        let mut node =
            unsafe { NonNull::<HeapNode>::new_unchecked(new_section.as_ptr() as *mut HeapNode) };
        unsafe { node.as_mut() }.next = None;
        unsafe { node.as_mut() }.size = new_section.capacity() * core::mem::size_of::<Page>();
        if self.head.is_none() {
            self.head = Some(node);
        } else {
            let mut elem = self.head.unwrap();
            while let Some(f) = unsafe { elem.as_ref() }.next {
                elem = f;
            }
            unsafe { elem.as_mut() }.next = Some(node);
        }
        new_section.leak();
        Ok(())
    }

    /// Perform an actual allocation
    fn run_alloc(&mut self, layout: core::alloc::Layout) -> *mut u8 {
        if self.head.is_none()
            && self
                .expand_with_physical_memory(layout.size() + layout.align())
                .is_err()
        {
            return core::ptr::null_mut();
        }

        let mut times = 0;
        loop {
            times += 1;
            let mut elem = self.head;
            let mut prev_elem: Option<NonNull<HeapNode>> = None;
            let mut best_fit_link: &mut Option<NonNull<HeapNode>> = &mut None;
            let mut best_fit: Option<NonNull<HeapNode>> = None;
            let mut best_fit_ha: Option<HeapNodeAlign> = None;

            while let Some(mut h) = elem {
                let ha = unsafe { h.as_mut() }.calc_alignment(layout.size(), layout.align());
                if ha.size_needed <= unsafe { h.as_ref() }.size {
                    if let Some(b) = best_fit {
                        if unsafe { h.as_ref() }.size < unsafe { b.as_ref() }.size {
                            best_fit_link = if let Some(mut pe) = prev_elem {
                                &mut unsafe { pe.as_mut() }.next
                            } else {
                                &mut self.head
                            };
                            best_fit = elem;
                            best_fit_ha = Some(ha);
                        }
                    } else {
                        best_fit_link = if let Some(mut pe) = prev_elem {
                            &mut unsafe { pe.as_mut() }.next
                        } else {
                            &mut self.head
                        };
                        best_fit = elem;
                        best_fit_ha = Some(ha);
                    }
                }
                prev_elem = elem;
                elem = unsafe { h.as_ref() }.next;
            }

            if let Some(best) = best_fit {
                let ha = best_fit_ha.unwrap();
                let r = if ha.pre_align < core::mem::size_of::<HeapNode>() {
                    if (unsafe { best.as_ref() }.size - ha.size_needed)
                        < core::mem::size_of::<HeapNode>()
                    {
                        //The entire block will be used
                        *best_fit_link = unsafe { best.as_ref() }.next;
                        (unsafe { best.as_ref() }.start() + ha.pre_align) as *mut u8
                    } else {
                        let after_node = unsafe { best.as_ref() }.start() + ha.size_needed;
                        let mut node = unsafe {
                            NonNull::<HeapNode>::new_unchecked(after_node as *mut HeapNode)
                        };
                        unsafe { node.as_mut() }.size =
                            unsafe { best.as_ref() }.size - ha.size_needed;
                        unsafe { node.as_mut() }.next =
                            unsafe { best_fit_link.unwrap().as_ref().next };
                        *best_fit_link = Some(node);
                        (unsafe { best.as_ref() }.start() + ha.pre_align) as *mut u8
                    }
                } else {
                    doors_macros2::kernel_print!(
                        "Alloc layout size {:x} alignment {:x}\r\n",
                        layout.size(),
                        layout.align()
                    );
                    doors_macros2::kernel_print!(
                        "A free node size {:x} will exist before the placement\r\n",
                        ha.pre_align
                    );
                    let b = unsafe { best.as_ref() };
                    b.print();
                    if (b.size - ha.size_needed) < core::mem::size_of::<HeapNode>() {
                        doors_macros2::kernel_print!("The end of the block will be used\r\n");
                    } else {
                        doors_macros2::kernel_print!(
                            "There will be blank space at the end of the block\r\n"
                        );
                    }
                    self.print();
                    todo!();
                };
                return r;
            }
            if times == 1
                && self
                    .expand_with_physical_memory(layout.size() + layout.align())
                    .is_err()
            {
                doors_macros2::kernel_print!("OUT OF MEMORY 1\r\n");
                return core::ptr::null_mut();
            }
            if times == 2 {
                doors_macros2::kernel_print!("OUT OF MEMORY 2\r\n");
                return core::ptr::null_mut();
            }
        }
    }

    /// Perform an actual deallocation
    fn run_dealloc(&mut self, ptr: *mut u8, layout: core::alloc::Layout) {
        let mut new_node = unsafe { HeapNode::with_ptr(ptr, layout) };
        let e = self.head.take();
        if let Some(e) = e {
            if (e.as_ptr() as usize) > (new_node.as_ptr() as usize) {
                // new node comes before head, it becomes the new head, and new node points to the old head
                unsafe { new_node.as_mut() }.next = Some(e);
                unsafe { new_node.as_mut() }.try_merge_with_next();
                self.head = Some(new_node);
            } else {
                // need to find where in the list the new node fits
                self.head = Some(e);
                let mut check = e;
                loop {
                    let checknext = unsafe { check.as_ref() }.next;
                    if let Some(cn) = checknext {
                        if (cn.as_ptr() as usize) > (new_node.as_ptr() as usize) {
                            // new node comes before the next element, insert it in between
                            unsafe { check.as_mut() }.next = Some(new_node);
                            unsafe { new_node.as_mut() }.next = Some(cn);
                            unsafe { check.as_mut() }.try_merge_with_next();
                            break;
                        } else {
                            //check further down the list
                            check = cn;
                        }
                    } else {
                        // This element is the only or last one, so the new node comes after this node
                        unsafe { check.as_mut() }.next = Some(new_node);
                        unsafe { check.as_mut() }.try_merge_with_next();
                        break;
                    }
                }
            }
        } else {
            // heap is empty, now the free node is the heap
            self.head = Some(new_node);
        }
        //TODO merge blocks if possible?
    }
}

unsafe impl core::alloc::GlobalAlloc for Locked<HeapManager<'_>> {
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

//! Memory management code that is common to both 32 and 64 bit x86.
//! The heap is a linked list of memory areas ([HeapNode]).
//! Memory is organized by blocks that represent free memory.

use core::{alloc::Layout, ptr::NonNull};

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

#[derive(Clone, Debug)]
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

    /// Print information for the node
    fn print(&self) {
        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
            "heap node is {:p} {:x?}\r\n",
            self,
            self
        ));
        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
            "\t {:?}\r\n",
            self.check()
        ));
    }

    /// Check the validity of the node
    fn check(&self) -> Result<(), ()> {
        if self.size == 0 {
            return Err(());
        }
        if let Some(n) = &self.next {
            let n = unsafe { n.as_ref() };
            if (self.start() + self.size) > crate::address(n) {
                return Err(());
            }
        }
        Ok(())
    }

    /// Returns the next Self, as an optional reference
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

    /// Create a heap node, at the specified location, with the specified size
    unsafe fn with_size(
        ptr: *mut u8,
        size: usize,
        next: Option<NonNull<HeapNode>>,
    ) -> NonNull<Self> {
        let node = ptr as *mut Self;
        (*node).size = size;
        (*node).next = next;
        NonNull::<Self>::new_unchecked(node)
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
        if let Some(r) = self.head {
            let mut t = unsafe { r.as_ref() };
            crate::VGA.print_str("HEAP:\r\n");
            t.print();
            while let Some(a) = t.next() {
                a.print();
                t = a;
            }
            while let Some(t2) = t.next() {
                t2.print();
            }
        } else {
            crate::VGA.print_str("Heap is empty\r\n");
        }
    }

    /// Check the heap
    pub fn check(&self) -> Result<(), ()> {
        if let Some(r) = self.head {
            let mut t = unsafe { r.as_ref() };
            t.check()?;
            while let Some(a) = t.next() {
                a.check()?;
                t = a;
            }
            while let Some(t2) = t.next() {
                t2.check()?;
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Expand the heap by a certain amount, using real memory.
    fn expand_with_physical_memory(&mut self, amount: usize) -> Result<NonNull<[u8]>, ()> {
        use core::alloc::Allocator as caAlloc;
        // Round up a partial page to a whole page, a is number of pages, not number of bytes
        let (a, r) = (
            amount / core::mem::size_of::<Page>(),
            amount % core::mem::size_of::<Page>(),
        );
        let a = if r != 0 { a + 1 } else { a };

        let layout =
            Layout::from_size_align(amount, core::mem::size_of::<Page>()).map_err(|_| ())?;
        let new_section = self.vmm.allocate(layout).map_err(|_| ())?;

        let sa = new_section.as_ptr() as *mut u8 as usize;
        let mut mm = self.mm.sync_lock();
        for i in (sa..sa + a * core::mem::size_of::<Page>()).step_by(core::mem::size_of::<Page>()) {
            mm.map_new_page(i)?;
        }
        drop(mm);

        let mut node =
            unsafe { NonNull::<HeapNode>::new_unchecked(new_section.as_ptr() as *mut HeapNode) };
        unsafe { node.as_mut() }.next = None;
        unsafe { node.as_mut() }.size = new_section.len();
        if self.head.is_none() {
            self.head = Some(node);
        } else {
            let mut elem = self.head.unwrap();
            while let Some(f) = unsafe { elem.as_ref() }.next {
                elem = f;
            }
            unsafe { elem.as_mut() }.next = Some(node);
        }
        Ok(new_section)
    }

    /// A function to provide some troubleshooting for memory management functions
    #[inline(never)]
    fn troubleshoot(&self, val: usize, val2: usize) {
        if doors_macros::config_check_equals!(mm_debug, "true")
            && crate::DEBUG_PRINT.load(core::sync::atomic::Ordering::SeqCst)
        {
            self.print();
            crate::VGA.sync_flush();
        }
        loop {
            core::hint::black_box(val);
            core::hint::black_box(val2);
        }
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
            let mut best_fit_prev: Option<NonNull<HeapNode>> = None;
            let mut best_fit: Option<NonNull<HeapNode>> = None;
            let mut best_fit_ha: Option<HeapNodeAlign> = None;

            while let Some(mut h) = elem {
                let ha = unsafe { h.as_mut() }.calc_alignment(layout.size(), layout.align());
                let size_really_needed = ha.pre_align + ha.size_needed;
                if size_really_needed <= unsafe { h.as_ref() }.size {
                    if let Some(b) = best_fit {
                        if unsafe { h.as_ref() }.size < unsafe { b.as_ref() }.size {
                            best_fit_link = if let Some(mut pe) = prev_elem {
                                &mut unsafe { pe.as_mut() }.next
                            } else {
                                &mut self.head
                            };
                            best_fit = elem;
                            best_fit_prev = prev_elem;
                            best_fit_ha = Some(ha);
                        }
                    } else {
                        best_fit_link = if let Some(mut pe) = prev_elem {
                            &mut unsafe { pe.as_mut() }.next
                        } else {
                            &mut self.head
                        };
                        best_fit = elem;
                        best_fit_prev = prev_elem;
                        best_fit_ha = Some(ha);
                    }
                }
                prev_elem = elem;
                elem = unsafe { h.as_ref() }.next;
            }

            if let Some(mut best) = best_fit {
                let best = unsafe { best.as_mut() };
                if doors_macros::config_check_equals!(mm_debug, "true")
                    && crate::DEBUG_PRINT.load(core::sync::atomic::Ordering::SeqCst)
                {
                    crate::VGA.print_str("BEST IS: ");
                    best.print();
                }
                let ha = best_fit_ha.unwrap();
                let r = if ha.pre_align < core::mem::size_of::<HeapNode>() {
                    if (best.size - ha.size_needed - ha.pre_align)
                        < core::mem::size_of::<HeapNode>()
                    {
                        //The entire block will be used
                        if doors_macros::config_check_equals!(mm_debug, "true")
                            && crate::DEBUG_PRINT.load(core::sync::atomic::Ordering::SeqCst)
                        {
                            crate::VGA.print_str("ALLOC1\r\n");
                        }
                        *best_fit_link = best.next;
                        (best.start() + ha.pre_align) as *mut u8
                    } else {
                        if doors_macros::config_check_equals!(mm_debug, "true")
                            && crate::DEBUG_PRINT.load(core::sync::atomic::Ordering::SeqCst)
                        {
                            crate::VGA.print_str("ALLOC2\r\n");
                        }
                        let after_node = best.start() + ha.size_needed + ha.pre_align;
                        let mut node = unsafe {
                            NonNull::<HeapNode>::new_unchecked(after_node as *mut HeapNode)
                        };
                        unsafe { node.as_mut() }.size = best.size - ha.size_needed;
                        unsafe { node.as_mut() }.next = best.next;
                        *best_fit_link = Some(node);
                        (best.start() + ha.pre_align) as *mut u8
                    }
                } else {
                    if (best.size - ha.size_needed - ha.pre_align)
                        < core::mem::size_of::<HeapNode>()
                    {
                        if doors_macros::config_check_equals!(mm_debug, "true")
                            && crate::DEBUG_PRINT.load(core::sync::atomic::Ordering::SeqCst)
                        {
                            crate::VGA.print_str("ALLOC3\r\n");
                        }
                        let mut prev = best_fit_prev.unwrap();
                        let prev = unsafe { prev.as_mut() };
                        prev.next = best.next;
                        (crate::address(best) + ha.pre_align) as *mut u8
                    } else {
                        if doors_macros::config_check_equals!(mm_debug, "true")
                            && crate::DEBUG_PRINT.load(core::sync::atomic::Ordering::SeqCst)
                        {
                            crate::VGA.print_str("ALLOC4\r\n");
                        }
                        let newblock = crate::address(best) + ha.pre_align + ha.size_needed;
                        if best.size < (ha.size_needed + ha.pre_align) {
                            self.troubleshoot(best.size, ha.size_needed + ha.pre_align);
                        }
                        let e = unsafe {
                            HeapNode::with_size(
                                newblock as *mut u8,
                                best.size - ha.size_needed - ha.pre_align,
                                best.next,
                            )
                        };
                        best.next = Some(e);
                        best.size = ha.pre_align;
                        (crate::address(best) + ha.pre_align) as *mut u8
                    }
                };
                if self.check().is_err() {
                    if doors_macros::config_check_equals!(mm_debug, "true")
                        && crate::DEBUG_PRINT.load(core::sync::atomic::Ordering::SeqCst)
                    {
                        crate::VGA.print_str("Failed ALLOC\r\n");
                    }
                    self.troubleshoot(42, 43);
                } else if doors_macros::config_check_equals!(mm_debug, "true")
                    && crate::DEBUG_PRINT.load(core::sync::atomic::Ordering::SeqCst)
                {
                    crate::VGA.print_str("Successfully ran ALLOC\r\n");
                    self.print();
                }
                return r;
            }
            if times == 1 {
                if self
                    .expand_with_physical_memory(layout.size() + layout.align())
                    .is_err()
                {
                    return core::ptr::null_mut();
                }
            }
            if times == 2 {
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
        if self.check().is_err() {
            if crate::DEBUG_PRINT.load(core::sync::atomic::Ordering::SeqCst) {
                crate::VGA.print_str("Failed DEALLOC\r\n");
            }
            self.troubleshoot(44, 45);
        } else if doors_macros::config_check_equals!(mm_debug, "true")
            && crate::DEBUG_PRINT.load(core::sync::atomic::Ordering::SeqCst)
        {
            crate::VGA.print_str("Successfully ran DEALLOC\r\n");
            self.print();
        }
        //TODO merge blocks if possible?
    }
}

unsafe impl core::alloc::GlobalAlloc for Locked<HeapManager<'_>> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut alloc = self.sync_lock();
        let layout2 = layout.align_to(HeapNode::NODEALIGN).unwrap();
        alloc.run_alloc(layout2)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        let mut alloc = self.sync_lock();
        let layout2 = layout.align_to(HeapNode::NODEALIGN).unwrap();
        alloc.run_dealloc(ptr, layout2);
    }
}

//! Memory management code

use core::ptr::NonNull;

use crate::Locked;

use crate::modules::video::TextDisplayTrait;

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
pub struct HeapNode {
    /// The optional next node of free memory for the heap
    next: Option<NonNull<HeapNode>>,
    /// The size of this node, including the size of this header
    size: usize,
}

impl HeapNode {
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
            post_align: posterr,
        }
    }
}

/// The heap manager for the system. It assumes it starts at a given address and expands to the end of known memory.
pub struct HeapManager {
    /// The beginning of the list of free memory nodes.
    head: [Option<NonNull<HeapNode>>; 3],
}

unsafe impl Send for HeapManager {}

impl HeapManager {
    /// Create a heap manager.
    pub const fn new() -> Self {
        Self {
            head: [None, None, None],
        }
    }

    /// Initialize the specified heap with an address and size
    pub fn init(&mut self, i: usize, addr: usize, size: usize) {
        if self.head[i].is_none() {
            doors_macros2::kernel_print!("Initing with {:x} bytes memory\r\n", size);
            let mut nn = unsafe { NonNull::new_unchecked(&mut *(addr as *mut HeapNode)) };
            unsafe { nn.as_mut() }.next = None;
            unsafe { nn.as_mut() }.size = size;
            self.head[i] = Some(nn);
        }
        else {
            doors_macros2::kernel_print!("NOT Initing with {:x} bytes memory\r\n", size);
        }
    }

    /// Print details of the heap
    pub fn print(&self) {
        for head in &self.head {
            if let Some(mut r) = head {
                doors_macros2::kernel_print!("head: {:p}\r\n", r);
                loop {
                    doors_macros2::kernel_print!(
                        "heap node is size {:x} {:x}\r\n",
                        unsafe { &*r.as_ptr() }.size,
                        unsafe { &*r.as_ptr() }.next_address() - 1
                    );

                    if let Some(nr) = unsafe { r.as_mut() }.next {
                        r = nr;
                    } else {
                        break;
                    }
                }
            } else {
                doors_macros2::kernel_print!("Heap is empty\r\n");
            }
        }
    }

    /// Perform an actual allocation
    fn run_alloc(&mut self, layout: core::alloc::Layout) -> *mut u8 {
        //parse each heap area separately
        for head in &mut self.head {
            if head.is_none() {
                continue;
            }

            let mut elem = *head;
            let mut prev_elem: Option<NonNull<HeapNode>> = None;
            let mut best_fit_link: &mut Option<NonNull<HeapNode>> = &mut None;
            let mut best_fit: Option<NonNull<HeapNode>> = None;
            let mut best_fit_ha: Option<HeapNodeAlign> = None;

            while let Some(mut h) = elem {
                //Calculate required alignment for the current node
                let ha = unsafe { h.as_mut() }.calc_alignment(layout.size(), layout.align());
                if ha.size_needed <= unsafe { h.as_ref() }.size {
                    if let Some(b) = best_fit {
                        if unsafe { h.as_ref() }.size < unsafe { b.as_ref() }.size {
                            best_fit_link = if let Some(mut pe) = prev_elem {
                                &mut unsafe { pe.as_mut() }.next
                            } else {
                                head
                            };
                            best_fit = elem;
                            best_fit_ha = Some(ha);
                        }
                    } else {
                        best_fit_link = if let Some(mut pe) = prev_elem {
                            &mut unsafe { pe.as_mut() }.next
                        } else {
                            head
                        };
                        best_fit = elem;
                        best_fit_ha = Some(ha);
                    }
                }
                prev_elem = elem;
                elem = unsafe { h.as_ref() }.next;
            }

            let retval = if let Some(best) = best_fit {
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
                    doors_macros2::kernel_print!("A free node will exist before the placement\r\n");
                    if (unsafe { best.as_ref() }.size - ha.size_needed)
                        < core::mem::size_of::<HeapNode>()
                    {
                        doors_macros2::kernel_print!("The end of the block will be used\r\n");
                    } else {
                        doors_macros2::kernel_print!(
                            "There will be blank space at the end of the block\r\n"
                        );
                    }
                    self.print();
                    unimplemented!();
                };
                r
            } else {
                continue;
            };
            return retval;
        }

        core::ptr::null_mut()
    }

    /// Perform an actual deallocation
    fn run_dealloc(&mut self, _ptr: *mut u8, _layout: core::alloc::Layout) {

        //TODO merge blocks if possible?
    }
}

unsafe impl core::alloc::GlobalAlloc for Locked<HeapManager> {
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

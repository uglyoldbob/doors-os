//! Arm cortex-m specific scheduler code.
//!
//! Context switching is built around the shared interrupt entry stub in
//! [`crate::modules::interrupt::arm::default_handler`]. Every task is interrupted by an
//! exception, and the stub leaves the following frame on the task's stack (growing down):
//!
//! ```text
//! [hardware exception frame: r0,r1,r2,r3,r12,lr,pc,xpsr]
//! [s16-s31]
//! [r4-r11, lr (exc_return)]
//! [dummy (duplicate of lr)]   <- the stack pointer after the stubs pushes (saved sp)
//! ```
//!
//! The saved `sp` of a thread points at the dummy slot (exactly what the stub recorded).
//! Restoring a thread simply moves the stack pointer there, pops the dummy, then the callee
//! saved registers (leaving the exception return value in lr), pops the fpu registers and
//! executes an exception return, which restores the rest of the state straight from the
//! hardware frame.

use alloc::vec::Vec;
use core::arch::naked_asm;

/// The exception return value used for fabricated new task frames. All code runs on the main
/// stack in thread mode with the floating point context managed by the shared interrupt stub.
const EXC_RETURN_MSP: u32 = 0xffff_fff9;

/// The value loaded into xpsr for newly created tasks, the thumb bit must always be set.
const INITIAL_XPSR: u32 = 0x0100_0000;

/// The page table data for a task. The cortex-m core has no mmu, so this is a no-op.
pub struct PageTableData();

impl PageTableData {
    /// Get the page table data of the currently running context
    pub fn from_current() -> Self {
        Self()
    }

    /// Install the page table data, if necessary
    pub fn install(&self) {}
}

/// The saved context for a thread
#[derive(Debug, Default)]
#[repr(C)]
pub struct Context {
    /// The stack pointer of the saved context, pointing at the callee saved register block
    /// that the shared interrupt entry stub pushed
    pub sp: u32,
    /// The spin lock nesting count of this context. Saved and restored with the context so a
    /// context that yields voluntarily while holding a spin lock doesn't corrupt the count of
    /// whichever context runs next.
    pub preempt: usize,
}

/// The context for a thread that lives on the stack
#[derive(Debug, Default)]
#[repr(C)]
pub struct StackContext {
    /// r0 register
    pub r0: u32,
    /// r1 register
    pub r1: u32,
    /// r2 register
    pub r2: u32,
    /// r3 register
    pub r3: u32,
    /// r12 register
    pub r12: u32,
    /// link register (from the hardware exception frame)
    pub lr: u32,
    /// program counter (from the hardware exception frame)
    pub pc: u32,
    /// program status register
    pub psr: u32,
    /// r4 register
    pub r4: u32,
    /// r5 register
    pub r5: u32,
    /// r6 register
    pub r6: u32,
    /// r7 register
    pub r7: u32,
    /// r8 register
    pub r8: u32,
    /// r9 register
    pub r9: u32,
    /// r10 register
    pub r10: u32,
    /// r11 register
    pub r11: u32,
}

/// The size of the stack for new tasks, in number of stack entries, not bytes.
const STACK_SIZE: usize = 2048;

impl super::Task {
    /// Create a new task
    pub fn new(_ptd: PageTableData, f: fn()) -> Self {
        let mut s = Stack::new(STACK_SIZE);
        let mut c = Context::default();
        // The cortex-m core requires the stack pointer to stay 8 byte aligned, so start the
        // fabricated frame from an aligned address.
        let mut sp = s.top() & !7u32;

        // The pushes below walk from the top of the stack downward, so the first push ends up
        // at the highest address. This has to produce exactly the frame the interrupt entry
        // stub leaves behind, where the hardware frame was pushed first and therefore lives at
        // the highest addresses, and the dummy slot the saved stack pointer points at is pushed
        // last, which puts it at the lowest address of the frame.

        // Fabricated hardware exception frame. The frame is unwound by the exception return,
        // so xpsr ends up at the top of it and r0 at the bottom.
        s.push(&mut sp, INITIAL_XPSR); // xpsr
        s.push(&mut sp, Self::task_trampoline as *const () as u32); // pc
        s.push(&mut sp, task_stuck as *const () as u32); // lr
        s.push(&mut sp, 0); // r12
        s.push(&mut sp, 0); // r3
        s.push(&mut sp, 0); // r2
        s.push(&mut sp, 0); // r1
        s.push(&mut sp, f as *const () as u32); // r0 (the trampoline takes the function in r0)
        for _s in (16..32).rev() {
            s.push(&mut sp, 0); // s16-s31
        }
        // The value the restore pops into lr, which is then used for the exception return.
        s.push(&mut sp, EXC_RETURN_MSP);
        for _r in (4..12).rev() {
            s.push(&mut sp, 0); // r4-r11
        }
        s.push(&mut sp, EXC_RETURN_MSP); // dummy slot
                                         // The saved context sp points at the dummy slot, the same value the stub records.
        c.sp = sp;

        Self {
            context: Some(c),
            status: super::TaskStatus::Runnable,
            page_table_data: _ptd,
            _f: Some(f),
            stack: Some(s),
        }
    }

    /// This function is the entry point of newly created threads, it runs the actual thread
    /// function and marks the task completed when it returns.
    extern "C" fn task_trampoline(f: u32) -> ! {
        let f: fn() = unsafe { core::mem::transmute::<u32, fn()>(f) };
        f();
        super::SCHEDULER.read().as_ref().unwrap().task_completed()
    }
}

/// A place for the link register of fabricated task frames to land if the trampoline ever
/// returns, which should never happen.
#[unsafe(naked)]
extern "C" fn task_stuck() -> ! {
    naked_asm!("1: b 1b")
}

impl Context {
    /// Read registers from the stack for a thread context
    pub fn stack_read(&self, stack: &Stack) -> (&Context, StackContext) {
        let mut sc = StackContext::default();
        // The stack pointer points at the dummy slot, so the callee saved block the entry stub
        // pushed sits directly above it.
        sc.r4 = stack.reference(self.sp + 4);
        sc.r5 = stack.reference(self.sp + 8);
        sc.r6 = stack.reference(self.sp + 12);
        sc.r7 = stack.reference(self.sp + 16);
        sc.r8 = stack.reference(self.sp + 20);
        sc.r9 = stack.reference(self.sp + 24);
        sc.r10 = stack.reference(self.sp + 28);
        sc.r11 = stack.reference(self.sp + 32);
        // Above the callee saved block sits the s16-s31 block, and above that the hardware
        // frame, which the exception entry pushed first and is therefore at the highest
        // addresses.
        let hw = self.sp + 4 + 36 + 64;
        sc.r0 = stack.reference(hw);
        sc.r1 = stack.reference(hw + 4);
        sc.r2 = stack.reference(hw + 8);
        sc.r3 = stack.reference(hw + 12);
        sc.r12 = stack.reference(hw + 16);
        sc.lr = stack.reference(hw + 20);
        sc.pc = stack.reference(hw + 24);
        sc.psr = stack.reference(hw + 28);
        (self, sc)
    }

    /// Restores thread context not on the stack, and the thread stack pointer.
    /// The pops here must exactly mirror the pushes of the shared interrupt entry stub in
    /// [`crate::modules::interrupt::arm::default_handler`]. The final `bx lr` exits the
    /// exception with the exception return value that was captured when this thread was
    /// interrupted, letting the hardware pop the rest of the state off of the threads stack.
    #[unsafe(naked)]
    pub(crate) unsafe extern "C" fn thread_restore(m: &Context) {
        naked_asm!(
            "\
            ldr r1, [r0]
            mov sp, r1
            pop {{r1}}
            pop {{r4-r11, lr}}
            vpop {{s16-s31}}
            bx lr"
        );
    }

    /// Saves thread context that is not already on the stack, and the thread stack pointer.
    /// The interrupt entry stub already pushed everything else and recorded the stack pointer
    /// in DOORS_IRQ_CTX.
    #[unsafe(naked)]
    pub(crate) unsafe extern "C" fn thread_save(m: &mut Context) {
        naked_asm!(
            "\
            ldr r1, =DOORS_IRQ_CTX
            ldr r1, [r1]
            str r1, [r0]
            bx lr"
        );
    }
}

/// Stack storage for a task
pub struct Stack {
    /// The actual stack
    data: Vec<u32>,
}

impl Stack {
    /// Construct a new Self
    fn new(size: usize) -> Self {
        Self {
            data: alloc::vec![0; size],
        }
    }

    /// A helper for building a stack from an existing stack
    pub fn helper(stack_start: usize, stack_size: usize) -> Vec<u32> {
        unsafe { Vec::from_raw_parts(stack_start as *mut u32, stack_size / 4, stack_size / 4) }
    }

    /// Construct a stack from an existing stack
    pub fn from_existing(data: Vec<u32>) -> Self {
        Self { data }
    }

    /// The top of the stack (one past the last entry)
    fn top(&self) -> u32 {
        (crate::slice_address(&self.data) + self.data.len() * core::mem::size_of::<u32>()) as u32
    }

    /// Retrieve an item from the stack by absolute address
    fn reference(&self, addr: u32) -> u32 {
        let a = addr as *const u32;
        unsafe { *a }
    }

    /// Update an item on the stack to the given value
    fn update(&mut self, addr: u32, val: u32) {
        let a = addr as *mut u32;
        unsafe { *a = val };
    }

    /// Push a value onto the stack
    fn push(&mut self, sp: &mut u32, val: u32) {
        *sp -= core::mem::size_of::<u32>() as u32;
        self.update(*sp, val);
    }
}

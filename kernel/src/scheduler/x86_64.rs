//! X86 64 bit specific scheduler code

use core::arch::naked_asm;

use alloc::vec::Vec;
use gdbstub_arch::x86::reg::X86_64CoreRegs;

/// The saved context for a thread
#[derive(Debug)]
#[repr(C)]
pub struct Context {
    /// rsp register
    pub rsp: u64,
    /// rbx register
    pub rbx: u64,
}

/// The context for a thread that lives on the stack
#[derive(Debug, Default)]
#[repr(C)]
pub struct StackContext {
    /// rax register
    pub rax: u64,
    /// rcx register
    pub rcx: u64,
    /// rdx register
    pub rdx: u64,
    /// r8 register
    pub r8: u64,
    /// r9 register
    pub r9: u64,
    /// r10 register
    pub r10: u64,
    /// r11 register
    pub r11: u64,
    /// r12 register
    pub r12: u64,
    /// r13 register
    pub r13: u64,
    /// r14 register
    pub r14: u64,
    /// r15 register
    pub r15: u64,
    /// rflags register
    pub rflags: u64,
    /// rsi register
    pub rsi: u64,
    /// rdi register
    pub rdi: u64,
    /// rbp register
    pub rbp: u64,
    /// rip register
    pub rip: u64,
}

/// The size of the stack for new tasks, in number of stack entries, not bytes.
const STACK_SIZE: usize = 1024;

impl super::Task {
    /// Create a new task
    pub fn new(f: fn()) -> Self {
        let mut s = Stack::new(STACK_SIZE);
        let mut sc = StackContext::default();
        let mut c = Context::new();
        s.set_rsp(&mut c.rsp);
        sc.rax = 0x64;
        sc.rcx = 0x65;
        sc.rdx = 0x66;
        sc.r8 = 0x67;
        sc.r9 = 0x68;
        sc.r10 = 0x69;
        sc.r11 = 0x6a;
        sc.r12 = 0x6b;
        sc.r13 = 0x6c;
        sc.r14 = 0x6d;
        sc.r15 = 0x6e;
        sc.rsi = 0x6f;
        sc.rdi = 0x70;
        sc.rip = 0x71;
        sc.rdi = f as *const () as u64;
        let start_eip = Self::task_runner as *const () as u64;
        s.push(&mut c.rsp, 0x10);
        let saved_rsp = c.rsp;
        s.push(&mut c.rsp, saved_rsp);
        s.push(&mut c.rsp, sc.rflags | 1 << 9);
        s.push(&mut c.rsp, 0x8);
        s.push(&mut c.rsp, start_eip as u64);
        s.push(&mut c.rsp, sc.rbp);
        s.push(&mut c.rsp, sc.r11);
        s.push(&mut c.rsp, sc.r10);
        s.push(&mut c.rsp, sc.r9);
        s.push(&mut c.rsp, sc.r8);
        s.push(&mut c.rsp, sc.rdi);
        s.push(&mut c.rsp, sc.rsi);
        s.push(&mut c.rsp, sc.rdx);
        s.push(&mut c.rsp, sc.rcx);
        s.push(&mut c.rsp, sc.rax);
        s.push(&mut c.rsp, 43);
        let t = irq_finisher as *const () as u64;
        s.push(&mut c.rsp, t); // mocked end of the irq handler
        s.push(&mut c.rsp, sc.rbp);
        s.push(&mut c.rsp, sc.r15);
        s.push(&mut c.rsp, sc.r14);
        s.push(&mut c.rsp, sc.r13);
        s.push(&mut c.rsp, sc.r12);
        s.push(&mut c.rsp, c.rbx);

        s.push(&mut c.rsp, thread_wrapper as *const () as u64); // the mocked return for the scheduler
        sc.rbp = c.rsp;
        let s = Self {
            context: Some(c),
            status: super::TaskStatus::Runnable,
            _f: Some(f),
            stack: Some(s),
        };
        s
    }

    /// This function runs extra threads in the kernel, ending them gracefully when they are done (eventually)
    fn task_runner(main_func: fn()) {
        main_func();
        super::SCHEDULER.read().as_ref().unwrap().task_completed();
        loop {
            use crate::kernel::SystemTrait;
            crate::SYSTEM.read().idle();
        }
    }
}

impl Context {
    /// Construct an empty context
    pub fn new() -> Self {
        Self { rbx: 98, rsp: 99 }
    }

    /// Write registers into the stack for a thread context
    pub fn stack_write(&self, stack: &mut Stack, regs: &X86_64CoreRegs) {
        let rsp = self.rsp + 0xa8;
        stack.update(rsp + 16, regs.regs[12]);
        stack.update(rsp + 24, regs.regs[13]);
        stack.update(rsp + 32, regs.regs[14]);
        stack.update(rsp + 40, regs.regs[15]);
        let rsp = self.rsp + 0xa8 + 0x58 + 16 + 8 + 0x48;
        stack.update(rsp, regs.regs[0]);
        stack.update(rsp + 8, regs.regs[2]);
        stack.update(rsp + 16, regs.regs[3]);
        stack.update(rsp + 24, regs.regs[4]);
        stack.update(rsp + 32, regs.regs[5]);
        stack.update(rsp + 40, regs.regs[8]);
        stack.update(rsp + 48, regs.regs[9]);
        stack.update(rsp + 56, regs.regs[10]);
        stack.update(rsp + 64, regs.regs[11]);
        stack.update(rsp + 72, regs.regs[12]);
        stack.update(rsp + 128, regs.rip);
        stack.update(rsp + 136, regs.eflags as u64);
    }

    /// Read registers from the stack for a thread context
    pub fn stack_read(&self, stack: &Stack) -> (&Context, StackContext) {
        let mut sc = StackContext::default();
        let rsp = self.rsp + 0xa8;
        //let rbx = stack.reference(rsp);
        sc.r12 = stack.reference(rsp + 16);
        sc.r13 = stack.reference(rsp + 24);
        sc.r14 = stack.reference(rsp + 32);
        sc.r15 = stack.reference(rsp + 40);
        //sc.rbp = stack.reference(rsp + 40);
        //let rsp = con.rsp + 0x120 + 56;
        //let rbx = stack.reference(rsp + 8);
        //sc.rbp = stack.reference(rsp + 16);
        let rsp = self.rsp + 0xa8 + 0x58 + 16 + 8 + 0x48;
        sc.rax = stack.reference(rsp);
        sc.rcx = stack.reference(rsp + 8);
        sc.rdx = stack.reference(rsp + 16);
        sc.rsi = stack.reference(rsp + 24);
        sc.rdi = stack.reference(rsp + 32);
        sc.r8 = stack.reference(rsp + 40);
        sc.r9 = stack.reference(rsp + 48);
        sc.r10 = stack.reference(rsp + 56);
        sc.r11 = stack.reference(rsp + 64);
        sc.rbp = stack.reference(rsp + 72);
        sc.rip = stack.reference(rsp + 128);
        sc.rflags = stack.reference(rsp + 136);
        (self, sc)
    }

    /// Saves thread context that is not already on the stack, and the thread stack pointer
    #[naked]
    pub(crate) unsafe extern "C" fn thread_restore(m: &Context) {
        naked_asm!(
            "\
            mov rsp, [rdi];\
            mov rbx, [rdi+8];\
            ret;"
        );
    }

    /// Restores thread context not on the stack, and the thread stack pointer
    #[naked]
    pub(crate) unsafe extern "C" fn thread_save(m: &mut Context) {
        naked_asm!(
            "\
            mov [rdi], rsp;\
            mov [rdi+8], rbx;\
            ret;"
        );
    }
}

/// Stack storage for a task
pub struct Stack {
    /// The actual stack
    data: Vec<u64>,
}

impl Stack {
    /// Construct a new Self
    fn new(size: usize) -> Self {
        let mut s = Vec::with_capacity(size);
        for _ in 0..size {
            s.push(0);
        }
        Self { data: s }
    }

    /// A helper for building a stack from an existing stack
    pub fn helper(stack_start: u64, stack_size: u64) -> Vec<u64> {
        unsafe {
            Vec::from_raw_parts(
                stack_start as *mut u64,
                stack_size as usize / 8,
                stack_size as usize / 8,
            )
        }
    }

    /// Construct a stack from an existing stack
    pub fn from_existing(data: Vec<u64>) -> Self {
        Self { data }
    }

    /// Retrieve an item from the stack by absolute address
    fn reference(&self, addr: u64) -> u64 {
        let a = addr as *const u64;
        unsafe { *a }
    }

    /// Update an item on the stack to the given value
    fn update(&mut self, addr: u64, val: u64) {
        let a = addr as *mut u64;
        unsafe { *a = val };
    }

    /// Set the rsp value to the end of the stack
    fn set_rsp(&self, rsp: &mut u64) {
        *rsp = (crate::slice_address(&self.data) + self.data.len() * core::mem::size_of::<u64>())
            as u64;
    }

    /// Push a value onto the stack
    fn push(&mut self, rsp: &mut u64, val: u64) {
        *rsp = *rsp - core::mem::size_of::<u64>() as u64;
        self.update(*rsp, val);
    }
}

/// The finishing function for irq handlers
#[naked]
pub(crate) unsafe extern "C" fn irq_finisher(irqnum: u8) -> ! {
    naked_asm!(
        "\
        add rsp, 8;\
        pop rax;\
        pop rcx;\
        pop rdx;\
        pop rsi;\
        pop rdi;\
        pop r8;\
        pop r9;\
        pop r10;\
        pop r11;\
        pop rbp;\
        iretq;"
    );
}

doors_macros::todo_item!("Get rid of this function and merge functionality into irq_finisher");
/// The thread wrapper function for starting a thread
#[naked]
pub(crate) unsafe extern "C" fn thread_wrapper() -> ! {
    naked_asm!(
        "\
        pop rbx;\
        pop r12;\
        pop r13;\
        pop r14;\
        pop r15;\
        pop rbp;\
        ret;"
    );
}

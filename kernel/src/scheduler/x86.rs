//! X86 32 bit specific scheduler code

use core::arch::naked_asm;

use crate::gdbstub::x86::reg::X86CoreRegs;
use alloc::vec::Vec;

/// The saved context for a thread
#[derive(Debug, Default)]
#[repr(C)]
pub struct Context {
    /// esp register
    pub esp: u32,
}

/// The context for a thread that lives on the stack
#[derive(Debug, Default)]
#[repr(C)]
pub struct StackContext {
    /// eax register
    pub eax: u32,
    /// ebx register
    pub ebx: u32,
    /// ecx register
    pub ecx: u32,
    /// edx register
    pub edx: u32,
    /// eflags register
    pub eflags: u32,
    /// rsi register
    pub esi: u32,
    /// edi register
    pub edi: u32,
    /// ebp register
    pub ebp: u32,
    /// eip register
    pub eip: u32,
}

/// The size of the stack for new tasks, in number of stack entries, not bytes.
const STACK_SIZE: usize = 1024;

impl super::Task {
    /// Create a new task
    pub fn new(f: fn()) -> Self {
        let mut s = Stack::new(STACK_SIZE);
        let mut sc = StackContext::default();
        let mut c = Context::default();
        s.set_rsp(&mut c.esp);
        sc.eax = f as *const () as u32;
        sc.ebx = 0x64;
        sc.ecx = 0x65;
        sc.edx = 0x66;
        sc.esi = 0x6f;
        sc.edi = 0x70;
        sc.eip = 0x71;
        sc.ebp = 0x72;
        sc.edi = 0x73;
        let start_eip = Self::task_runner as *const () as u32;
        s.push(&mut c.esp, sc.eflags | 1 << 9);
        s.push(&mut c.esp, 8);
        s.push(&mut c.esp, start_eip as u32);
        s.push(&mut c.esp, sc.ebp);
        s.push(&mut c.esp, sc.edi);
        s.push(&mut c.esp, sc.esi);
        s.push(&mut c.esp, sc.edx);
        s.push(&mut c.esp, sc.ecx);
        s.push(&mut c.esp, sc.eax);
        s.push(&mut c.esp, sc.ebx);

        s.push(&mut c.esp, irq_finisher as *const () as u32); // the mocked return for the scheduler
        sc.ebp = c.esp;
        let s = Self {
            context: Some(c),
            status: super::TaskStatus::Runnable,
            _f: Some(f),
            stack: Some(s),
        };
        s
    }

    /// This function runs extra threads in the kernel, ending them gracefully when they are done (eventually)
    fn task_runner() {
        unsafe { core::arch::asm!("call eax;") };
        super::SCHEDULER.read().as_ref().unwrap().task_completed();
    }
}

impl Context {
    /// Write registers into the stack for a thread context
    pub fn stack_write(&self, stack: &mut Stack, regs: &X86CoreRegs) {
        //let rsp = self.esp + 0x120;
        let rsp = self.esp + 0x120 + 96 + 56 + 24;
        stack.update(rsp, regs.eax);
        stack.update(rsp + 8, regs.ecx);
        stack.update(rsp + 16, regs.edx);
        stack.update(rsp + 24, regs.esi);
        stack.update(rsp + 32, regs.edi);
        stack.update(rsp + 128, regs.eip);
        stack.update(rsp + 136, regs.eflags as u32);
    }

    /// Read registers from the stack for a thread context
    pub fn stack_read(&self, stack: &Stack) -> (&Context, StackContext) {
        let mut sc = StackContext::default();
        //let rsp = self.esp + 0x120;
        //let rbx = stack.reference(rsp);
        //sc.rbp = stack.reference(rsp + 40);
        //let rsp = con.rsp + 0x120 + 56;
        //let rbx = stack.reference(rsp + 8);
        //sc.rbp = stack.reference(rsp + 16);
        let rsp = self.esp + 0x120 + 96 + 56 + 24;
        sc.eax = stack.reference(rsp);
        sc.ecx = stack.reference(rsp + 8);
        sc.edx = stack.reference(rsp + 16);
        sc.esi = stack.reference(rsp + 24);
        sc.edi = stack.reference(rsp + 32);
        sc.ebp = stack.reference(rsp + 72);
        sc.eip = stack.reference(rsp + 128);
        sc.eflags = stack.reference(rsp + 136);
        (self, sc)
    }

    /// Restores thread context not on the stack, and the thread stack pointer
    #[naked]
    pub(crate) unsafe extern "C" fn thread_restore(m: &Context) {
        naked_asm!(
            "\
            mov esp, [eax];\
            ret;"
        );
    }

    /// Saves thread context that is not already on the stack, and the thread stack pointer    
    #[naked]
    pub(crate) unsafe extern "C" fn thread_save(m: &mut Context) {
        naked_asm!(
            "\
            mov [eax], esp;\
            ret;"
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
        let mut s = Vec::with_capacity(size);
        for _ in 0..size {
            s.push(0);
        }
        Self { data: s }
    }

    /// A helper for building a stack from an existing stack
    pub fn helper(stack_start: usize, stack_size: usize) -> Vec<u32> {
        unsafe { Vec::from_raw_parts(stack_start as *mut u32, stack_size / 4, stack_size / 4) }
    }

    /// Construct a stack from an existing stack
    pub fn from_existing(data: Vec<u32>) -> Self {
        Self { data }
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

    /// Set the rsp value to the end of the stack
    fn set_rsp(&self, rsp: &mut u32) {
        *rsp = (crate::slice_address(&self.data) + self.data.len() * core::mem::size_of::<u32>())
            as u32;
    }

    /// Push a value onto the stack
    fn push(&mut self, rsp: &mut u32, val: u32) {
        *rsp = *rsp - core::mem::size_of::<u32>() as u32;
        self.update(*rsp, val);
    }
}

/// The finishing function for irq handlers
#[naked]
pub(crate) unsafe extern "C" fn irq_finisher(irqnum: u8) -> ! {
    naked_asm!(
        "\
        pop ebx;\
        pop eax;\
        pop ecx;\
        pop edx;\
        pop esi;\
        pop edi;\
        pop ebp;\
        iretd;"
    );
}

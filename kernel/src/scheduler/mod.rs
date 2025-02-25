//! Code for the task/thread scheduler of the kernel.

/// The saved context for a thread
#[derive(Debug, Clone)]
#[repr(C)]
pub struct Context {
    /// rbp register
    pub rbp: u64,
    /// rax register
    pub rax: u64,
    /// rbx register
    pub rbx: u64,
    /// rcx register
    pub rcx: u64,
    /// rdx register
    pub rdx: u64,
    /// rsi register
    pub rsi: u64,
    /// rdi register
    pub rdi: u64,
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
}

core::arch::global_asm!(include_str!("x86.s"));

extern "C" {
    fn thread_save(m: &mut Context);
    fn thread_restore(m: &Context);
}

impl Context {
    fn new() -> Self {
        Self {
            rbp: 0,
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rflags: 0,
        }
    }

    /// Experimental code to save a thread context
    pub fn save(c: &mut Context) {
        unsafe { thread_save(c) };
    }

    /// Experimental code to restore a thread context
    pub fn restore(&self) {
        unsafe { thread_restore(self) };
    }
}

/// The status of a task in the kernel
pub enum TaskStatus {
    /// The thread is not currently executing and there is a saved context
    WithContext(Context),
    /// The thread is running so there is no context
    WithoutContext(),
    /// The thread is brand new and has not run yet
    New(),
}

/// A general purpose task or thread in the kernel
pub struct Task {
    /// The status of the task    
    status: TaskStatus,
}

impl Task {
    /// A test function for checking the operation of context saving and restoring
    #[inline(never)]
    pub fn test() {
        let mut c = Context::new();
        Context::save(&mut c);
        crate::VGA.print_str(&alloc::format!(
            "The context stored is {:p} {:x?}\r\n",
            &c,
            c
        ));
        crate::VGA.sync_flush();
        c.restore();
    }
}

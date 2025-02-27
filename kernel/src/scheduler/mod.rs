//! Code for the task/thread scheduler of the kernel.

use alloc::vec::Vec;
use spin::RwLock;

use crate::{
    kernel::SystemTrait,
    modules::timer::{TimerInstance, TimerInstanceInner, TimerTrait},
    Arc, IrqGuarded, IrqGuardedInner, IrqGuardedUse,
};

/// The saved context for a thread
#[derive(Debug)]
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
    /// rsp register
    pub rsp: u64,
}

core::arch::global_asm!(include_str!("x86.s"));

extern "C" {
    fn thread_save(m: &mut Context);
    fn thread_restore(m: &Context);
    fn thread_wrapper1();
    fn thread_wrapper2();
}

impl Context {
    /// Construct an empty context
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
            rflags: 1 << 9,
            rsp: 0,
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

doors_macros::todo_item!("Create a guard page for stack");

/// Stack storage for a task
struct Stack {
    /// The actual stack
    data: Vec<u64>,
    /// The index into the stack
    index: usize,
}

impl Stack {
    /// Construct a new Self
    fn new(size: usize) -> Self {
        let mut s = Vec::with_capacity(size);
        for _ in 0..size {
            s.push(0);
        }
        Self {
            data: s,
            index: size,
        }
    }

    /// Set the rsp value to the end of the stack
    fn set_rsp(&self, rsp: &mut u64) {
        *rsp = (crate::slice_address(&self.data) + self.data.len() * core::mem::size_of::<u64>())
            as u64;
    }

    /// Push a value onto the stack
    fn push(&mut self, rsp: &mut u64, val: u64) {
        *rsp = *rsp - core::mem::size_of::<u64>() as u64;
        self.index -= 1;
        self.data[self.index] = val;
    }
}

/// A general purpose task or thread in the kernel
pub struct Task {
    /// The context of the task    
    context: Option<Context>,
    /// The initial function of the task
    f: Option<fn()>,
    /// The thread stack
    stack: Option<Stack>,
}

doors_macros::todo_item!("Figure out a way to lock a task onto a specific processor?");

/// The size of the stack for new tasks, in number of stack entries, not bytes.
const STACK_SIZE: usize = 1024;

impl Task {
    /// Print the task
    pub fn print(&self) {
        crate::VGA.print_str(&alloc::format!("Context is {:x?}\r\n", self.context));
    }

    /// Create a new task
    pub fn new(f: fn()) -> Self {
        let mut s = Stack::new(STACK_SIZE);
        let mut c = Context::new();
        s.set_rsp(&mut c.rsp);
        let start_eip = f as *const () as u64;
        s.push(&mut c.rsp, 0x10);
        let saved_rsp = c.rsp;
        s.push(&mut c.rsp, saved_rsp);
        s.push(&mut c.rsp, c.rflags);
        s.push(&mut c.rsp, 0x8);
        s.push(&mut c.rsp, start_eip as u64);
        s.push(&mut c.rsp, c.rbp);
        s.push(&mut c.rsp, c.r11);
        s.push(&mut c.rsp, c.r10);
        s.push(&mut c.rsp, c.r9);
        s.push(&mut c.rsp, c.r8);
        s.push(&mut c.rsp, c.rdi);
        s.push(&mut c.rsp, c.rsi);
        s.push(&mut c.rsp, c.rdx);
        s.push(&mut c.rsp, c.rcx);
        s.push(&mut c.rsp, c.rbx);
        s.push(&mut c.rsp, c.rax);
        s.push(&mut c.rsp, 42);
        s.push(&mut c.rsp, 42);
        s.push(&mut c.rsp, thread_wrapper2 as *const () as u64); // mocked end of the irq handler
        s.push(&mut c.rsp, c.rbp);
        s.push(&mut c.rsp, c.r14);
        s.push(&mut c.rsp, c.rbx);
        for _ in 0..12 {
            s.push(&mut c.rsp, 42);
        }
        s.push(&mut c.rsp, thread_wrapper1 as *const () as u64); // the mocked return for the scheduler
        c.rbp = c.rsp;
        let s = Self {
            context: Some(c),
            f: Some(f),
            stack: Some(s),
        };
        s
    }

    /// Construct a new task from the currently running function
    const fn running() -> Self {
        Self {
            context: None,
            f: None,
            stack: None,
        }
    }

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

/// The scheduler object
pub static SCHEDULER: RwLock<Option<Scheduler>> = RwLock::new(None);

/// The actual contents of a scheduler
pub struct InnerScheduler {
    /// The list of tasks local to the scheduler
    local_tasks: Vec<Task>,
    /// The currently executing task
    cur_task: Task,
    /// The timer instance for the scheduler
    timer: Option<TimerInstance>,
}

impl InnerScheduler {
    /// Create a new scheduler
    pub const fn new() -> Self {
        Self {
            local_tasks: Vec::new(),
            cur_task: Task::running(),
            timer: None,
        }
    }

    /// Print all tasks
    pub fn print(&self) {
        crate::VGA.print_str(&alloc::format!(
            "There are {} tasks\r\n",
            self.local_tasks.len()
        ));
        for t in &self.local_tasks {
            t.print();
        }
        crate::VGA.sync_flush();
    }
}

/// The struct shared to the interrupt handler
pub struct SchedulerProtected(IrqGuarded<InnerScheduler>);

/// The thread scheduler for the kernel
pub struct Scheduler {
    /// The protected data
    i: Arc<SchedulerProtected>,
}

impl Scheduler {
    /// Construct a new scheduler
    pub fn new() -> Self {
        let com = IrqGuardedInner::new(0, false, |_| {}, |_| {});
        let i = IrqGuarded::new(InnerScheduler::new(), &com);
        Self {
            i: Arc::new(SchedulerProtected(i)),
        }
    }

    /// The interrupt handler for the timer
    #[inline(never)]
    fn handle_interrupt(
        this: &Arc<SchedulerProtected>,
        mut timer: IrqGuardedUse<TimerInstanceInner>,
    ) {
        use crate::modules::timer::TimerInstanceInnerTrait;
        let mut this = this.0.interrupt_access();
        if let Some(mut task) = this.local_tasks.pop() {
            let new_context = match task.context.take() {
                Some(c) => c,
                None => {
                    todo!();
                }
            };
            core::mem::swap(&mut this.cur_task, &mut task);
            let mut old_context = Context::new();
            Context::save(&mut old_context);
            if let Some(_c) = task.context.replace(old_context) {
                panic!();
            }
            this.local_tasks.push(task);
            drop(this);
            timer.start_oneshot();
            new_context.restore();
        } else {
            timer.start_oneshot();
        }
    }

    /// Setup the timer
    pub fn timer_setup(&self) {
        use crate::modules::timer::TimerInstanceInnerTrait;
        let s2 = self.i.clone();
        let irqnum = self.i.0.irq();
        crate::SYSTEM.read().disable_irq(irqnum);
        {
            let mut this = self.i.0.interrupt_access();
            let mut t = crate::kernel::TIMERS.sync_lock();
            let timer = t.module(0);
            let mut t2 = timer.sync_lock();
            let mut t3 = t2.get_timer(0).unwrap();
            t3.register_handler(move |timer| Self::handle_interrupt(&s2, timer));
            crate::SYSTEM.read().disable_irq(irqnum);
            this.timer.replace(t3);
        }
        crate::SYSTEM.read().enable_irq(irqnum);
        {
            let this = self.i.0.sync_access();
            this.timer.as_ref().unwrap().sync_use().start_oneshot();
        }
    }

    /// Add a task
    pub fn add_task(&self, task: Task) {
        let mut this = self.i.0.sync_access();
        this.local_tasks.push(task);
    }

    /// Print all tasks
    pub fn print(&self) {
        let this = self.i.0.sync_access();
        this.print();
    }
}

//! Code for the task/thread scheduler of the kernel.

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use spin::RwLock;

use crate::{
    kernel::SystemTrait,
    modules::timer::{TimerInstance, TimerInstanceInner, TimerTrait},
    Arc, IrqGuarded, IrqGuardedInner, IrqGuardedUse, Locked, NotSafeForInterrupts,
    SafeForInterrupts, TaskId,
};

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

core::arch::global_asm!(include_str!("x86.s"));

extern "C" {
    fn thread_save(m: &mut Context);
    fn thread_restore(m: &Context);
    fn thread_wrapper1();
}

impl Context {
    /// Construct an empty context
    fn new() -> Self {
        Self { rbx: 98, rsp: 99 }
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

    /// Retrieve an item from the stack by absolute address
    fn reference(&self, addr: u64) -> u64 {
        let a = addr as *const u64;
        unsafe { *a }
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

/// The current status of a task in the kernel
#[derive(PartialEq)]
enum TaskStatus {
    /// The task can run
    Runnable,
    /// The task has completed
    Completed,
}

/// A general purpose task or thread in the kernel
pub struct Task {
    /// The context of the task
    context: Option<Context>,
    /// The status of the task
    status: TaskStatus,
    /// The initial function of the task
    _f: Option<fn()>,
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

    /// Look at the context for a task (primarily for a debugger)
    pub fn examine_context(&self) -> Option<&Context> {
        self.context.as_ref()
    }

    /// Look up any required registers on the stack
    pub fn examine_stack(&self) -> Option<(&Context, StackContext)> {
        if let Some(stack) = &self.stack {
            self.context.as_ref().map(|con| {
                let mut sc = StackContext::default();
                let rsp = con.rsp + 0x1c8;
                sc.rax = stack.reference(rsp + 8);
                sc.rcx = stack.reference(rsp + 16);
                sc.rdx = stack.reference(rsp + 24);
                sc.rsi = stack.reference(rsp + 32);
                sc.rdi = stack.reference(rsp + 40);
                sc.rbp = stack.reference(rsp + 48);
                sc.r8 = stack.reference(rsp + 56);
                sc.r9 = stack.reference(rsp + 64);
                sc.r10 = stack.reference(rsp + 72);
                sc.r11 = stack.reference(rsp + 80);
                sc.r12 = stack.reference(rsp + 88);
                sc.r13 = stack.reference(rsp + 96);
                sc.r14 = stack.reference(rsp + 104);
                sc.r15 = stack.reference(rsp + 112);
                sc.rip = stack.reference(rsp + 120);
                sc.rflags = stack.reference(rsp + 128);
                (con, sc)
            })
        } else {
            None
        }
    }

    /// This function runs extra threads in the kernel, ending them gracefully when they are done (eventually)
    fn task_runner(main_func: fn()) {
        main_func();
        SCHEDULER.read().as_ref().unwrap().task_completed();
        loop {}
    }

    /// Create a new task
    pub fn new(f: fn()) -> Self {
        let mut s = Stack::new(STACK_SIZE);
        let mut sc = StackContext::default();
        let mut c = Context::new();
        s.set_rsp(&mut c.rsp);
        sc.rax = 100;
        sc.rcx = 101;
        sc.rdx = 102;
        sc.r8 = 103;
        sc.r9 = 104;
        sc.r10 = 105;
        sc.r11 = 106;
        sc.r12 = 107;
        sc.r13 = 108;
        sc.r14 = 109;
        sc.r15 = 110;
        sc.rsi = 111;
        sc.rdi = 112;
        sc.rip = 113;
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
        let t = crate::boot::x86::boot64::irq_finisher as *const () as u64;
        s.push(&mut c.rsp, t); // mocked end of the irq handler
        s.push(&mut c.rsp, sc.rbp);
        s.push(&mut c.rsp, sc.r15);
        s.push(&mut c.rsp, sc.r14);
        s.push(&mut c.rsp, sc.r13);
        s.push(&mut c.rsp, sc.r12);
        s.push(&mut c.rsp, c.rbx);

        s.push(&mut c.rsp, thread_wrapper1 as *const () as u64); // the mocked return for the scheduler
        sc.rbp = c.rsp;
        let s = Self {
            context: Some(c),
            status: TaskStatus::Runnable,
            _f: Some(f),
            stack: Some(s),
        };
        s
    }

    /// Construct a new task from the currently running function
    const fn running() -> Self {
        Self {
            context: None,
            status: TaskStatus::Runnable,
            _f: None,
            stack: None,
        }
    }
}

/// The scheduler object
pub static SCHEDULER: RwLock<Option<Scheduler>> = RwLock::new(None);

/// The actual contents of a scheduler
pub struct InnerScheduler {
    /// The list of tasks local to the scheduler
    local_tasks: BTreeMap<TaskId, Task>,
    /// The currently executing task
    cur_task: Task,
    /// The id of the currently executing task
    cur_task_id: Option<TaskId>,
    /// The timer instance for the scheduler
    timer: Option<TimerInstance>,
}

impl InnerScheduler {
    /// Create a new scheduler
    pub const fn new() -> Self {
        Self {
            local_tasks: BTreeMap::new(),
            cur_task: Task::running(),
            cur_task_id: None,
            timer: None,
        }
    }

    /// Create an iterator over all tasks for this scheduler
    pub fn iter(&self) -> alloc::collections::btree_map::Iter<TaskId, Task> {
        self.local_tasks.iter()
    }

    /// Try to get thread details by thread id
    pub fn lookup(&self, id: TaskId) -> Option<&Task> {
        self.local_tasks.get(&id)
    }

    /// Print all tasks
    pub fn print(&self) {
        crate::VGA.print_str(&alloc::format!(
            "There are {} tasks\r\n",
            self.local_tasks.len()
        ));
        for (_id, t) in &self.local_tasks {
            doors_macros::todo_item!("Use the task id in the print function");
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
        let com = IrqGuardedInner::new(0, false, true, |_| {}, |_| {});
        let i = IrqGuarded::new(InnerScheduler::new(), &com);
        Self {
            i: Arc::new(SchedulerProtected(i)),
        }
    }

    /// Access the scheduler data from a synchronous non-interrupt context
    pub fn sync_access(&self) -> IrqGuardedUse<InnerScheduler, NotSafeForInterrupts> {
        self.i.0.sync_access()
    }

    /// Set the status of the current task to completed
    fn task_completed(&self) {
        let mut this = self.i.0.sync_access();
        this.cur_task.status = TaskStatus::Completed;
    }

    /// The interrupt handler for the timer
    #[inline(never)]
    fn handle_interrupt(
        this: &Arc<SchedulerProtected>,
        mut timer: IrqGuardedUse<TimerInstanceInner, SafeForInterrupts>,
    ) {
        use crate::modules::timer::TimerInstanceInnerTrait;
        let mut this = this.0.interrupt_access();

        loop {
            if this.local_tasks.is_empty() {
                timer.start_oneshot();
                drop(timer);
                return;
            }
            let taskid = this.local_tasks.keys().next().cloned().unwrap();
            let (taskid, mut task) = this.local_tasks.remove_entry(&taskid).unwrap();
            if TaskStatus::Runnable == task.status {
                let new_context = match task.context.take() {
                    Some(c) => c,
                    None => {
                        todo!();
                    }
                };
                core::mem::swap(&mut this.cur_task, &mut task);
                let mut old_task = this.cur_task_id.replace(taskid);
                if old_task.is_none() {
                    old_task.replace(TaskId::new());
                }
                let taskid = old_task.unwrap();
                let mut old_context = Context::new();
                Context::save(&mut old_context);
                if let Some(_c) = task.context.replace(old_context) {
                    panic!();
                }
                this.local_tasks.insert(taskid, task);
                drop(this);
                timer.start_oneshot();
                drop(timer);
                new_context.restore();
                return;
            }
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
        this.local_tasks.insert(TaskId::new(), task);
    }

    /// Print all tasks
    pub fn print(&self) {
        let this = self.i.0.sync_access();
        this.print();
    }
}

//! Code for the task/thread scheduler of the kernel.

use core::arch::naked_asm;

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use gdbstub_arch::x86::reg::X86_64CoreRegs;
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

impl Context {
    /// Construct an empty context
    fn new() -> Self {
        Self { rbx: 98, rsp: 99 }
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

doors_macros::todo_item!("Create a guard page for stack");

/// Stack storage for a task
struct Stack {
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

    /// Construct a stack from an existing stack
    fn from_existing(data: Vec<u64>) -> Self {
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

    /// Update register contents for a thread that is not currently running
    pub fn write_registers(&mut self, regs: &X86_64CoreRegs) -> Result<(), ()> {
        if let Some(stack) = &mut self.stack {
            if let Some(con) = &mut self.context {
                let rsp = con.rsp + 0x120;
                stack.update(rsp + 8, regs.regs[12]);
                stack.update(rsp + 16, regs.regs[13]);
                stack.update(rsp + 24, regs.regs[14]);
                stack.update(rsp + 32, regs.regs[15]);
                let rsp = con.rsp + 0x120 + 96 + 56 + 24;
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
                Ok(())
            } else {
                Err(())
            }
        } else {
            Err(())
        }
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
                let rsp = con.rsp + 0x120;
                //let rbx = stack.reference(rsp);
                sc.r12 = stack.reference(rsp + 8);
                sc.r13 = stack.reference(rsp + 16);
                sc.r14 = stack.reference(rsp + 24);
                sc.r15 = stack.reference(rsp + 32);
                //sc.rbp = stack.reference(rsp + 40);
                //let rsp = con.rsp + 0x120 + 56;
                //let rbx = stack.reference(rsp + 8);
                //sc.rbp = stack.reference(rsp + 16);
                let rsp = con.rsp + 0x120 + 96 + 56 + 24;
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
        let t = Self::irq_finisher as *const () as u64;
        s.push(&mut c.rsp, t); // mocked end of the irq handler
        s.push(&mut c.rsp, sc.rbp);
        s.push(&mut c.rsp, sc.r15);
        s.push(&mut c.rsp, sc.r14);
        s.push(&mut c.rsp, sc.r13);
        s.push(&mut c.rsp, sc.r12);
        s.push(&mut c.rsp, c.rbx);

        s.push(&mut c.rsp, Self::thread_wrapper as *const () as u64); // the mocked return for the scheduler
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

    /// Try to get mutable thread details by thread id
    pub fn lookup_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.local_tasks.get_mut(&id)
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

    /// Retrieve the task id of the current task
    pub fn cur_task_id(&self) -> Option<TaskId> {
        let mut this = self.i.0.sync_access();
        this.cur_task_id
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
                unsafe { Context::thread_save(&mut old_context) };
                if let Some(_c) = task.context.replace(old_context) {
                    panic!();
                }
                this.local_tasks.insert(taskid, task);
                drop(this);
                timer.start_oneshot();
                drop(timer);
                return unsafe { Context::thread_restore(&new_context) };
            }
        }
    }

    /// Setup the timer and start scheduling tasks with the timer
    pub fn timer_setup(&self, stack_start: u64, stack_size: u64) {
        use crate::modules::timer::TimerInstanceInnerTrait;
        let s2 = self.i.clone();
        let irqnum = self.i.0.irq();
        crate::SYSTEM.read().disable_irq(irqnum);
        {
            let mut this = self.i.0.interrupt_access();
            {
                let stack = unsafe {
                    Vec::from_raw_parts(
                        stack_start as *mut u64,
                        stack_size as usize / 8,
                        stack_size as usize / 8,
                    )
                };
                this.cur_task.stack.replace(Stack::from_existing(stack));
            }
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
    pub fn add_task(&self, task: Task) -> TaskId {
        let mut this = self.i.0.sync_access();
        let tid = TaskId::new();
        this.local_tasks.insert(tid, task);
        tid
    }

    /// Print all tasks
    pub fn print(&self) {
        let this = self.i.0.sync_access();
        this.print();
    }
}

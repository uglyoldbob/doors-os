//! Code for the task/thread scheduler of the kernel.

#[cfg(target_arch = "x86")]
use crate::gdbstub::x86::reg::X86CoreRegs;
#[cfg(target_arch = "x86_64")]
use crate::gdbstub::x86::reg::X86_64CoreRegs;
#[cfg(target_arch = "x86_64")]
mod x86_64;
use futures::StreamExt;
#[cfg(target_arch = "x86_64")]
use x86_64::*;
#[cfg(target_arch = "x86")]
mod x86;
#[cfg(target_arch = "x86")]
use x86::*;

use alloc::{boxed::Box, vec::Vec};
use spin::RwLock;

use crate::{
    kernel::SystemTrait,
    modules::timer::{TimerInstance, TimerInstanceTrait, TimerTrait, WeakTimerInstance},
    Arc, IrqGuarded, IrqGuardedInner, IrqGuardedUse, IrqNumbers, IrqStreamReader, IrqStreamWriter,
    NotSafeForInterrupts, SafeForInterrupts, TaskId,
};

doors_macros::todo_item!("Create a guard page for stack");

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
    /// The page table data
    page_table_data: PageTableData,
    /// The status of the task
    status: TaskStatus,
    /// The initial function of the task
    _f: Option<fn()>,
    /// The thread stack
    stack: Option<Stack>,
}

doors_macros::todo_item!("Figure out a way to lock a task onto a specific processor?");

impl Task {
    /// Print the task
    pub fn print(&self) {
        crate::VGA.print_str(&alloc::format!("Context is {:x?}\r\n", self.context));
    }

    #[cfg(target_arch = "x86_64")]
    /// Update register contents for a thread that is not currently running
    pub fn write_registers(&mut self, regs: &X86_64CoreRegs) -> Result<(), ()> {
        if let Some(stack) = &mut self.stack {
            if let Some(con) = &mut self.context {
                con.stack_write(stack, regs);
                Ok(())
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    #[cfg(target_arch = "x86")]
    /// Update register contents for a thread that is not currently running
    pub fn write_registers(&mut self, regs: &X86CoreRegs) -> Result<(), ()> {
        if let Some(stack) = &mut self.stack {
            if let Some(con) = &mut self.context {
                con.stack_write(stack, regs);
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
            self.context.as_ref().map(|con| con.stack_read(stack))
        } else {
            None
        }
    }

    /// Construct a new task from the currently running function
    fn running() -> Self {
        Self {
            context: None,
            status: TaskStatus::Runnable,
            page_table_data: PageTableData::from_current(),
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
    local_tasks: Vec<(TaskId, Task)>,
    /// The currently executing task
    cur_task: (TaskId, Task),
    /// The next task index to swap to
    next_task_index: usize,
    /// The timer instance for the scheduler
    timer: Option<TimerInstance>,
    /// Completed tasks
    completed: IrqStreamWriter<(crate::common::TaskId, Task)>,
    /// Receiver for completed tasks
    task_killer: Option<IrqStreamReader<(crate::common::TaskId, Task)>>,
}

impl InnerScheduler {
    /// Create a new scheduler, using the specified task id as the starting task
    pub fn new(id: TaskId, com: &IrqGuardedInner) -> Self {
        let cq = crate::common::new_irq_stream(&com, 10, 10);
        Self {
            local_tasks: Vec::new(),
            cur_task: (id, Task::running()),
            next_task_index: 0,
            timer: None,
            completed: cq.1,
            task_killer: Some(cq.0),
        }
    }

    /// Create an iterator over all tasks for this scheduler
    pub fn iter(&self) -> core::slice::Iter<'_, (TaskId, Task)> {
        self.local_tasks.iter()
    }

    /// Try to get thread details by thread id
    pub fn lookup(&self, id: TaskId) -> Option<&(TaskId, Task)> {
        self.local_tasks.iter().find(|a| a.0 == id)
    }

    /// Try to get mutable thread details by thread id
    pub fn lookup_mut(&mut self, id: TaskId) -> Option<&mut (TaskId, Task)> {
        self.local_tasks.iter_mut().find(|a| a.0 == id)
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
    pub fn new(id: TaskId) -> Self {
        let com = IrqGuardedInner::new(IrqNumbers::Only1(0), false, true, |_| {}, |_| {});
        let i = IrqGuarded::new(InnerScheduler::new(id, &com), &com);
        Self {
            i: Arc::new(SchedulerProtected(i)),
        }
    }

    /// Access the scheduler data from a synchronous non-interrupt context
    pub fn sync_access(&self) -> IrqGuardedUse<'_, InnerScheduler, NotSafeForInterrupts> {
        self.i.0.sync_access()
    }

    /// Set the status of the current task to completed
    fn task_completed(&self) -> ! {
        {
            let mut this = self.i.0.sync_access();
            this.cur_task.1.status = TaskStatus::Completed;
        }
        loop {
            crate::idle();
        }
    }

    /// Retrieve the task id of the current task
    pub fn cur_task_id(&self) -> TaskId {
        let this = self.i.0.sync_access();
        this.cur_task.0
    }

    /// A panic helper for the scheduler
    #[inline(never)]
    fn panic(val: usize) -> ! {
        loop {
            core::hint::black_box(val);
        }
    }

    /// The interrupt handler for the timer
    #[inline(never)]
    fn handle_interrupt(this: &Arc<SchedulerProtected>, timer: WeakTimerInstance) {
        use crate::modules::timer::TimerInstanceTrait;
        let mut this = this.0.interrupt_access();

        loop {
            if this.local_tasks.is_empty() {
                if let Some(t) = timer.upgrade() {
                    t.start_oneshot();
                }
                drop(timer);
                return;
            }
            let next_task_index = this.next_task_index;
            let t: &mut InnerScheduler = &mut this;
            if t.cur_task.1.context.is_some() {
                Self::panic(1);
            }
            match t.local_tasks[next_task_index].1.status {
                TaskStatus::Runnable => {
                    if (t.next_task_index + 2) < t.local_tasks.len() {
                        t.next_task_index += 1;
                    } else {
                        t.next_task_index = 0;
                    }
                    t.local_tasks[next_task_index].1.page_table_data.install();
                    let new_context = match t.local_tasks[next_task_index].1.context.take() {
                        Some(c) => c,
                        None => Self::panic(2),
                    };
                    let mut old_context = Context::default();
                    unsafe { Context::thread_save(&mut old_context) };
                    if t.cur_task.1.context.replace(old_context).is_some() {
                        Self::panic(3);
                    }
                    core::mem::swap(&mut t.local_tasks[next_task_index], &mut t.cur_task);
                    drop(this);
                    if let Some(t) = timer.upgrade() {
                        t.start_oneshot();
                    }
                    drop(timer);
                    return unsafe { Context::thread_restore(&new_context) };
                }
                TaskStatus::Completed => {
                    let a = this.local_tasks.swap_remove(next_task_index);
                    this.completed.push_interrupt(a);
                    crate::nop();
                    crate::nop();
                    crate::nop();
                    crate::nop();
                    crate::nop();
                    crate::nop();
                }
            }
        }
    }

    /// Setup the timer and start scheduling tasks with the timer
    pub fn timer_setup(&self, stack_start: usize, stack_size: usize) {
        use crate::modules::timer::TimerInstanceTrait;
        let s2 = self.i.clone();
        let irqnums = self.i.0.irqs();
        for i in irqnums {
            crate::SYSTEM.read().disable_irq(i);
        }
        {
            let mut this = self.i.0.interrupt_access();
            {
                let stack = Stack::helper(stack_start, stack_size);
                this.cur_task.1.stack.replace(Stack::from_existing(stack));
            }
            let mut t = crate::kernel::TIMERS.sync_lock();
            let timer = t.module(0);
            let mut t2 = timer.sync_lock();
            let t3 = t2
                .get_timer(
                    0,
                    100,
                    crate::modules::timer::TimerCallbackWithUsage::Multiple(Arc::new(Box::new(
                        move |timer| Self::handle_interrupt(&s2, timer),
                    ))),
                )
                .unwrap();
            this.timer.replace(t3);
        }
        let irqnums = self.i.0.irqs();
        for i in irqnums {
            crate::SYSTEM.read().enable_irq(i);
        }
        {
            let mut this = self.i.0.sync_access();
            this.timer.as_mut().unwrap().start_oneshot();
        }
    }

    /// Spawn a new thread
    pub fn spawn_thread(&self, t: fn()) {
        let task = Task::new(PageTableData::from_current(), t);
        self.add_task(task);
    }

    /// Start the task terminator
    pub async fn task_terminator(&self) {
        let mut this = self.i.0.sync_access();
        if let Some(mut tk) = this.task_killer.take() {
            let _ = crate::executor::spawn(async move {
                while let Some(t) = tk.next().await {
                    drop(t);
                }
            });
        }
    }

    /// Yield the current task
    pub fn yield_task(&self) {
        let this = self.i.0.sync_access();
        if let Some(timer) = &this.timer {
            timer.manually_trigger();
        }
    }

    /// Add a task
    fn add_task(&self, task: Task) -> TaskId {
        let mut this = self.i.0.sync_access();
        let tid = TaskId::default();
        this.local_tasks.push((tid, task));
        tid
    }

    /// Print all tasks
    pub fn print(&self) {
        let this = self.i.0.sync_access();
        this.print();
    }
}

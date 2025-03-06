//! Code for the task/thread scheduler of the kernel.

use alloc::{
    collections::{btree_map::BTreeMap, VecDeque},
    vec::Vec,
};
#[cfg(target_arch = "x86")]
use gdbstub_arch::x86::reg::X86CoreRegs;
#[cfg(target_arch = "x86_64")]
use gdbstub_arch::x86::reg::X86_64CoreRegs;
use spin::RwLock;

#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
use x86_64::*;

#[cfg(target_arch = "x86")]
mod x86;
#[cfg(target_arch = "x86")]
use x86::*;

use crate::{
    kernel::SystemTrait,
    modules::timer::{TimerInstance, TimerInstanceInner, TimerTrait},
    Arc, IrqGuarded, IrqGuardedInner, IrqGuardedUse, NotSafeForInterrupts, SafeForInterrupts,
    TaskId,
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
    local_tasks: VecDeque<(TaskId, Task)>,
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
            local_tasks: VecDeque::new(),
            cur_task: Task::running(),
            cur_task_id: None,
            timer: None,
        }
    }

    /// Create an iterator over all tasks for this scheduler
    pub fn iter(&self) -> alloc::collections::vec_deque::Iter<(TaskId, Task)> {
        self.local_tasks.iter()
    }

    /// Try to get thread details by thread id
    pub fn lookup(&self, id: TaskId) -> Option<&(TaskId, Task)> {
        self.local_tasks.get(id.value())
    }

    /// Try to get mutable thread details by thread id
    pub fn lookup_mut(&mut self, id: TaskId) -> Option<&mut (TaskId, Task)> {
        self.local_tasks.get_mut(id.value())
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
        let this = self.i.0.sync_access();
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
            let (taskid, mut task) = this.local_tasks.pop_front().unwrap();
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
                this.local_tasks.push_back((taskid, task));
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
                let stack = Stack::helper(stack_start, stack_size);
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
        this.local_tasks.push_back((tid, task));
        tid
    }

    /// Print all tasks
    pub fn print(&self) {
        let this = self.i.0.sync_access();
        this.print();
    }
}

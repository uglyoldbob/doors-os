//! This module holds code for the async executor used in the kernel.
//! TODO: use a kernel config to specify the size of waker queues

use core::task::Waker;

use crate::kernel::SystemTrait;
use crate::modules::video::TextDisplayTrait;

/// The id for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(usize);

impl TaskId {
    fn new() -> Self {
        static NEXT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
        Self(NEXT.fetch_add(1, core::sync::atomic::Ordering::Relaxed))
    }
}

/// A task for the kernel
pub struct Task {
    id: TaskId,
    future: core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()>>>,
}

impl Task {
    /// Construct a new task with a future.
    /// TODO determine a way to remove the 'static lifetime from the future
    pub fn new(future: impl core::future::Future<Output = ()> + 'static) -> Self {
        Self {
            id: TaskId::new(),
            future: alloc::boxed::Box::pin(future),
        }
    }

    /// Poll the task
    fn poll(&mut self, context: &mut core::task::Context) -> core::task::Poll<()> {
        self.future.as_mut().poll(context)
    }
}

type TaskListType<T> = crossbeam::queue::ArrayQueue<T>;

struct TaskListWaker {
    id: TaskId,
    tasks: alloc::sync::Arc<TaskListType<TaskId>>,
}

impl TaskListWaker {
    fn new(id: TaskId, tasks: alloc::sync::Arc<TaskListType<TaskId>>) -> Waker {
        Waker::from(alloc::sync::Arc::new(Self { id, tasks }))
    }

    /// wakeup the task.
    /// TODO handle error for the push?
    fn wake_task(&self) {
        self.tasks.push(self.id);
    }
}

impl alloc::task::Wake for TaskListWaker {
    fn wake(self: alloc::sync::Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &alloc::sync::Arc<Self>) {
        self.wake_task();
    }
}

struct TaskList {
    tasks: alloc::sync::Arc<TaskListType<TaskId>>,
}

impl Default for TaskList {
    fn default() -> Self {
        Self {
            tasks: alloc::sync::Arc::new(TaskListType::new(100)),
        }
    }
}

impl TaskList {
    fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    fn add(&mut self, taskid: TaskId) -> Result<(), ()> {
        self.tasks.push(taskid).map_err(|_| ())
    }

    fn run(
        &mut self,
        system: &mut crate::kernel::System,
        all_tasks: &mut alloc::collections::BTreeMap<TaskId, Task>,
        wakers: &mut alloc::collections::BTreeMap<TaskId, Waker>,
    ) {
        while let Some(taskid) = self.tasks.pop() {
            let task = all_tasks.get_mut(&taskid);
            if let Some(task) = task {
                let waker = wakers
                    .entry(taskid)
                    .or_insert_with(|| TaskListWaker::new(taskid, self.tasks.clone()));
                let mut context = core::task::Context::from_waker(waker);
                match task.poll(&mut context) {
                    core::task::Poll::Ready(()) => {
                        all_tasks.remove(&taskid);
                        wakers.remove(&taskid);
                    }
                    core::task::Poll::Pending => {}
                }
            }
        }
    }
}

/// The async executor for the kernel
#[derive(Default)]
pub struct Executor {
    all_tasks: alloc::collections::BTreeMap<TaskId, Task>,
    wakers: alloc::collections::BTreeMap<TaskId, Waker>,
    basic_tasks: TaskList,
}

impl Executor {
    /// Spawn a new task
    pub fn spawn(&mut self, task: Task) -> Result<(), ()> {
        let id = task.id;
        if self.all_tasks.insert(id, task).is_some() {
            panic!("Task already spawned");
        }
        self.basic_tasks.add(id)
    }

    /// Runs tasks
    fn run_tasks(&mut self, system: &mut crate::kernel::System) {
        self.basic_tasks
            .run(system, &mut self.all_tasks, &mut self.wakers);
    }

    /// Run the executor
    pub fn run(&mut self, mut system: crate::kernel::System) -> ! {
        doors_macros2::kernel_print!("Running the executor\r\n");
        let mut l = 0usize;
        loop {
            doors_macros2::kernel_print!("Running the executor loop {}\r\n", l);
            self.run_tasks(&mut system);
            system.idle_if(|| self.basic_tasks.is_empty());
            l += 1;
        }
    }
}

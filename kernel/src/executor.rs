//! This module holds code for the async executor used in the kernel.
//! TODO: use a kernel config to specify the size of waker queues

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use alloc::borrow::ToOwned;

/// The id for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(usize);

impl TaskId {
    /// Construct the next unique task id
    fn new() -> Self {
        static NEXT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
        Self(NEXT.fetch_add(1, core::sync::atomic::Ordering::Relaxed))
    }
}

/// An example struct that is non sendable
pub struct NonSendable {
    /// The non-sendable element
    elem: alloc::rc::Rc<u32>,
}

impl NonSendable {
    /// Construct a new Self
    pub fn new() -> Self {
        Self {
            elem: alloc::rc::Rc::new(0),
        }
    }

    /// Do the thing
    pub fn do_thing(&mut self) {
        self.elem = (*self.elem + 1).into();
    }
}

/// A task for the kernel
pub struct LocalTask<'a> {
    /// The id for the task. This is unique across all tasks in the system.
    id: TaskId,
    /// The future that the task executes
    future: core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + 'a>>,
}

impl<'a> LocalTask<'a> {
    /// Construct a new task with a future.
    pub fn new(future: impl core::future::Future<Output = ()> + 'a) -> Self {
        Self {
            id: TaskId::new(),
            future: alloc::boxed::Box::pin(future),
        }
    }

    /// Poll the task
    fn poll(&mut self, context: &mut core::task::Context) -> core::task::Poll<()> {
        self.future.as_mut().poll(context)
    }

    /// Yield the task to other tasks in the same priority
    pub async fn yield_now() {
        /// Yield implementation
        struct YieldNow {
            /// Has the task already yielded?
            yielded: bool,
        }

        impl Future for YieldNow {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                if self.yielded {
                    return Poll::Ready(());
                }
                self.yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }

        YieldNow { yielded: false }.await;
    }
}

/// A task for the kernel
pub struct Task<'a> {
    /// The id for the task. This is unique across all tasks in the system.
    id: TaskId,
    /// The future that the task executes
    future: core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + Send + 'a>>,
    /// Number of times it has been polled
    polled: usize,
}

impl<'a> Task<'a> {
    /// Construct a new task with a future.
    pub fn new(future: impl core::future::Future<Output = ()> + Send + 'a) -> Self {
        Self {
            id: TaskId::new(),
            future: alloc::boxed::Box::pin(future),
            polled: 0,
        }
    }

    /// Poll the task
    fn poll(&mut self, context: &mut core::task::Context) -> core::task::Poll<()> {
        self.future.as_mut().poll(context)
    }

    /// Yield the task to other tasks in the same priority
    pub async fn yield_now() {
        /// Yield implementation
        struct YieldNow {
            /// Has the task already yielded?
            yielded: bool,
        }

        impl Future for YieldNow {
            type Output = ();

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                if self.yielded {
                    return Poll::Ready(());
                }
                self.yielded = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }

        YieldNow { yielded: false }.await;
    }
}

/// Convenience type for the storage and processing of task ids in a task list.
type TaskListType<T> = crossbeam::queue::ArrayQueue<T>;

/// A waker for a task in a task list
struct TaskListWaker {
    /// The task id of the task to wake
    id: TaskId,
    /// The list of tasks of the associated list
    tasks: alloc::sync::Arc<TaskListType<TaskId>>,
}

impl TaskListWaker {
    /// Construct a new Self for the specified task and task list
    fn new(id: TaskId, tasks: alloc::sync::Arc<TaskListType<TaskId>>) -> Waker {
        Waker::from(alloc::sync::Arc::new(Self { id, tasks }))
    }

    /// wakeup the task.
    /// TODO handle error for the push?
    fn wake_task(&self) {
        crate::VGA2.print_str("Waking a task");
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

/// A list of tasks to be executed
pub struct TaskList {
    /// The list of task ids associated with the list
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
    /// Is the list empty?
    fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Add a task id to the list
    fn add(&mut self, taskid: TaskId) -> Result<(), ()> {
        self.tasks.push(taskid).map_err(|_| ())
    }

    /// Copy the number of times that tasks have been polled
    fn copy_polls(&mut self, taskid: TaskId, task: &Task<'_>, polled: &mut [Option<usize>; 6]) {
        if taskid.0 < polled.len() {
            polled[taskid.0] = Some(task.polled);
        }
        if task.polled > 1000 {
            loop {}
        }
    }

    /// Run tasks in the list
    fn run(
        &mut self,
        all_tasks: &mut alloc::collections::BTreeMap<TaskId, Task>,
        wakers: &mut alloc::collections::BTreeMap<TaskId, Waker>,
        polled: &mut [Option<usize>; 6],
    ) {
        while let Some(taskid) = self.tasks.pop() {
            let task = all_tasks.get_mut(&taskid);
            if let Some(task) = task {
                let waker = wakers
                    .entry(taskid)
                    .or_insert_with(|| TaskListWaker::new(taskid, self.tasks.clone()));
                let mut context = core::task::Context::from_waker(waker);
                task.polled += 1;
                crate::VGA2.print_str(&alloc::format!("Polling a task {}\r\n", taskid.0));
                self.copy_polls(taskid, task, polled);
                match task.poll(&mut context) {
                    core::task::Poll::Ready(()) => {
                        all_tasks.remove(&taskid);
                        wakers.remove(&taskid);
                    }
                    core::task::Poll::Pending => {}
                }
            } else {
                crate::VGA2.print_str(&alloc::format!("Task {} is gone\r\n", taskid.0));
            }
        }
    }
}

/// The async executor for the kernel
#[derive(Default)]
pub struct Executor<'a> {
    /// The list of all tasks in the executor
    all_tasks: alloc::collections::BTreeMap<TaskId, Task<'a>>,
    /// The list of all tasks specific to this executor
    local_tasks: alloc::collections::BTreeMap<TaskId, LocalTask<'a>>,
    /// The list of wakers for all tasks
    wakers: alloc::collections::BTreeMap<TaskId, Waker>,
    /// The basic list of tasks for the executor
    basic_tasks: TaskList,
    /// The number of times that tasks have been polled
    polled: [Option<usize>; 6],
}

impl<'a> Executor<'a> {
    /// Spawn a new task that always runs on this executor
    pub fn spawn_local(&mut self, task: LocalTask<'a>) -> Result<(), ()> {
        let id = task.id;
        if self.local_tasks.insert(id, task).is_some() {
            panic!("Task already spawned");
        }
        self.basic_tasks.add(id)
    }

    /// Spawn a task using a closure
    pub fn spawn_closure_local<F>(&mut self, c: F) -> Result<(), ()>
    where
        F: AsyncFnOnce() -> (),
        F::CallOnceFuture: 'a,
    {
        let task = LocalTask::new(c.async_call_once(()));
        self.spawn_local(task)
    }

    /// Spawn a new task
    pub fn spawn(&mut self, task: Task<'a>) -> Result<(), ()> {
        let id = task.id;
        if self.all_tasks.insert(id, task).is_some() {
            panic!("Task already spawned");
        }
        self.basic_tasks.add(id)
    }

    /// Spawn a task using a closure
    pub fn spawn_closure<F>(&mut self, c: F) -> Result<(), ()>
    where
        F: AsyncFnOnce() -> (),
        F::CallOnceFuture: Send + 'a,
    {
        let task = Task::new(c.async_call_once(()));
        self.spawn(task)
    }

    /// Runs tasks
    fn run_tasks(&mut self) {
        self.basic_tasks
            .run(&mut self.all_tasks, &mut self.wakers, &mut self.polled);
    }

    /// Get the polls for all tasks
    fn get_polls(&mut self) {
        for p in &mut self.polled {
            *p = None;
        }
        for (id, task) in self.all_tasks.iter() {
            if id.0 < self.polled.len() {
                self.polled[id.0] = Some(task.polled);
            }
        }
    }

    /// Run the executor
    pub fn run(&mut self) -> ! {
        let sys = crate::SYSTEM.sync_lock().to_owned().unwrap();
        loop {
            self.run_tasks();
            self.get_polls();
            use crate::kernel::SystemTrait;
            sys.idle_if(|| self.basic_tasks.is_empty());
        }
    }
}

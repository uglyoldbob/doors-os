//! This module holds code for the async executor used in the kernel.
//! TODO: use a kernel config to specify the size of waker queues

use alloc::collections::BTreeMap;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};

use crate::TaskId;

/// An example struct that is non sendable
pub struct NonSendable {
    /// The non-sendable element
    elem: alloc::rc::Rc<u32>,
}

impl Default for NonSendable {
    fn default() -> Self {
        Self {
            elem: alloc::rc::Rc::new(0),
        }
    }
}

impl NonSendable {
    /// Do the thing
    pub fn do_thing(&mut self) {
        self.elem = (*self.elem + 1).into();
    }
}

doors_macros::todo_item!("Actually use the asynchronous local tasks");
/// A task for the kernel
#[allow(unused)]
pub struct LocalAsyncTask<'a> {
    /// The id for the task. This is unique across all tasks in the system.
    id: TaskId,
    /// The future that the task executes
    future: core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + 'a>>,
}

impl<'a> LocalAsyncTask<'a> {
    /// Construct a new task with a future.
    pub fn new(future: impl core::future::Future<Output = ()> + 'a) -> Self {
        Self {
            id: TaskId::default(),
            future: alloc::boxed::Box::pin(future),
        }
    }

    /// Poll the task
    #[allow(unused)]
    fn poll(&mut self, context: &mut core::task::Context) -> core::task::Poll<()> {
        self.future.as_mut().poll(context)
    }

    /// Yield the task to other tasks in the same priority
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
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
pub struct AsyncTask<'a> {
    /// The id for the task. This is unique across all tasks in the system.
    id: TaskId,
    /// The future that the task executes
    future: core::pin::Pin<alloc::boxed::Box<dyn core::future::Future<Output = ()> + Send + 'a>>,
    /// Number of times it has been polled
    polled: usize,
}

impl<'a> AsyncTask<'a> {
    /// Construct a new task with a future.
    pub fn new(future: impl core::future::Future<Output = ()> + Send + 'a) -> Self {
        Self {
            id: TaskId::default(),
            future: alloc::boxed::Box::pin(future),
            polled: 0,
        }
    }

    /// Poll the task
    fn poll(&mut self, context: &mut core::task::Context) -> core::task::Poll<()> {
        self.future.as_mut().poll(context)
    }

    /// Yield the task to other tasks in the same priority
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
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
    /// Construct a new waker for the specified task and task list
    fn new_waker(id: TaskId, tasks: alloc::sync::Arc<TaskListType<TaskId>>) -> Waker {
        Waker::from(alloc::sync::Arc::new(Self { id, tasks }))
    }

    /// wakeup the task.
    /// TODO handle error for the push?
    fn wake_task(&self) {
        let _ = self.tasks.push(self.id);
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
    fn add(&self, taskid: TaskId) -> Result<(), ()> {
        self.tasks.push(taskid).map_err(|_| ())
    }

    /// Pop a task from the list
    fn pop(&self) -> Option<TaskId> {
        self.tasks.pop()
    }

    /// Copy the number of times that tasks have been polled
    fn copy_polls(
        &mut self,
        taskid: TaskId,
        task: &AsyncTask<'_>,
        polled: &mut [Option<usize>; 6],
    ) {
        if taskid.0 < polled.len() {
            polled[taskid.0] = Some(task.polled);
        }
    }

    /// Run tasks in the list
    fn run(
        &mut self,
        all_tasks: &mut alloc::collections::BTreeMap<TaskId, AsyncTask>,
        wakers: &mut alloc::collections::BTreeMap<TaskId, Waker>,
        polled: &mut [Option<usize>; 6],
        current_task_id: &crate::LockedArc<Option<TaskId>>,
    ) {
        while let Some(taskid) = self.tasks.pop() {
            let task = all_tasks.get_mut(&taskid);
            if let Some(task) = task {
                let waker = wakers
                    .entry(taskid)
                    .or_insert_with(|| TaskListWaker::new_waker(taskid, self.tasks.clone()));
                let mut context = core::task::Context::from_waker(waker);
                task.polled += 1;
                self.copy_polls(taskid, task, polled);
                *current_task_id.sync_lock() = Some(task.id);
                match task.poll(&mut context) {
                    core::task::Poll::Ready(()) => {
                        all_tasks.remove(&taskid);
                        wakers.remove(&taskid);
                    }
                    core::task::Poll::Pending => {}
                }
                *current_task_id.sync_lock() = None;
            }
        }
    }
}

/// The async executor for the kernel
#[derive(Default)]
pub struct Executor<'a> {
    /// The list of all tasks in the executor
    all_tasks: alloc::collections::BTreeMap<TaskId, AsyncTask<'a>>,
    /// The list of all tasks specific to this executor
    local_tasks: alloc::collections::BTreeMap<TaskId, LocalAsyncTask<'a>>,
    /// The list of wakers for all tasks
    wakers: alloc::collections::BTreeMap<TaskId, Waker>,
    /// The basic list of tasks for the executor
    basic_tasks: TaskList,
    /// The number of times that tasks have been polled
    polled: [Option<usize>; 6],
}

impl<'a> Executor<'a> {
    /// Spawn a new task that always runs on this executor
    pub fn spawn_local<F: Future<Output = ()> + 'a>(&mut self, task: F) -> Result<(), ()> {
        let task = LocalAsyncTask::new(task);
        let id = task.id;
        if self.local_tasks.insert(id, task).is_some() {
            panic!("Task already spawned");
        }
        self.basic_tasks.add(id)
    }

    /// Spawn a task using a future
    pub fn spawn_closure_local<F>(&mut self, c: F) -> Result<(), ()>
    where
        F: Future<Output = ()> + 'a,
    {
        self.spawn_local(c)
    }

    /// Spawn a new task
    fn spawn_task(&mut self, task: AsyncTask<'a>) -> Result<(), ()> {
        let id = task.id;
        if self.all_tasks.insert(id, task).is_some() {
            panic!("Task already spawned");
        }
        self.basic_tasks.add(id)
    }

    /// Spawn a future
    pub fn spawn<F>(&mut self, c: F) -> Result<(), ()>
    where
        F: Future<Output = ()> + Send + 'a,
    {
        let task = AsyncTask::new(c);
        self.spawn_task(task)
    }

    /// Runs tasks
    fn run_tasks(&mut self, cur: &crate::LockedArc<Option<TaskId>>) {
        self.basic_tasks
            .run(&mut self.all_tasks, &mut self.wakers, &mut self.polled, cur);
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

    /// Is the task list empty
    pub fn task_list_empty(&self) -> bool {
        self.basic_tasks.is_empty()
    }

    /// Run the executor
    pub fn run(&mut self, cur: &crate::LockedArc<Option<TaskId>>) {
        self.run_tasks(cur);
        self.get_polls();
    }
}

/// Represents the global executor for the kernel
/// This will become more complicated when multi-processor support is added.
pub struct GlobalExecutor {
    executor: crate::LockedArc<Executor<'static>>,
    /// The list of all tasks specific to this executor
    local_tasks: crate::LockedArc<alloc::collections::BTreeMap<TaskId, AsyncTask<'static>>>,
    /// The current task
    current_task_id: crate::LockedArc<Option<TaskId>>,
    /// Data stored for each task
    task_data:
        crate::AsyncLockedArc<BTreeMap<TaskId, doors_macros2::backtrace::location::Location>>,
}

impl GlobalExecutor {
    /// Build a new Global Executor
    pub fn new(executor: Executor<'static>) -> Self {
        Self {
            executor: crate::LockedArc::new(executor),
            local_tasks: crate::LockedArc::new(alloc::collections::BTreeMap::new()),
            current_task_id: crate::LockedArc::new(None),
            task_data: crate::AsyncLockedArc::new(BTreeMap::new()),
        }
    }

    /// Run the global executor
    pub fn run(&self) -> ! {
        loop {
            {
                let mut e = self.executor.sync_lock();
                e.run(&self.current_task_id);
            }
            {
                let e = self.executor.sync_lock();
                let t = self.local_tasks.sync_lock();
                crate::idle_if(|| e.task_list_empty() && t.is_empty());
            }
            {
                let mut e = self.executor.sync_lock();
                let mut t = self.local_tasks.sync_lock();
                if let Some(t) = t.pop_first() {
                    e.all_tasks.insert(t.0, t.1);
                    e.basic_tasks.add(t.0);
                }
            }
        }
    }

    /// Get the currently running async task id
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    pub async fn get_current_task_id(&self) -> Option<TaskId> {
        *self.current_task_id.sync_lock()
    }

    #[cfg(feature = "backtrace")]
    /// Register location data for a task
    pub async fn register_task_location(&self, td: doors_macros2::backtrace::location::Location) {
        if let Some(t) = *self.current_task_id.sync_lock() {
            self.task_data.sync_lock().insert(t, td);
        }
    }

    /// Print all of the task locations
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    pub async fn print_locations(&self) {
        let p = self.task_data.sync_lock();
        crate::VGA.print_str("ASYNC TASK DUMP FOLLOWS\r\n");
        for data in p.iter() {
            crate::VGA.print_str(&alloc::format!("{:?}\r\n", data));
        }
        crate::VGA.print_str("ASYNC TASK DUMP PRECEDES\r\n");
    }

    /// Spawn a new async task
    pub fn spawn<F: Future<Output = ()> + Send + 'static>(&self, task: F) -> Result<(), ()> {
        let mut e = self.local_tasks.sync_lock();
        let task = AsyncTask::new(task);
        let id = task.id;
        if e.insert(id, task).is_some() {
            return Err(());
        }
        Ok(())
    }
}

/// Get the currently running async task id
#[cfg_attr(feature = "backtrace", doors_macros::framed)]
pub async fn get_current_task_id() -> Option<TaskId> {
    crate::kernel::EXECUTOR
        .read()
        .as_ref()
        .unwrap()
        .get_current_task_id()
        .await
}

#[cfg(feature = "backtrace")]
/// Register location data for a task
pub async fn register_location(td: doors_macros2::backtrace::location::Location) {
    let e = crate::kernel::EXECUTOR.read();
    let e = e.as_ref().unwrap();
    e.register_task_location(td).await
}

/// Spawn a new async task
pub fn spawn<F: Future<Output = ()> + Send + 'static>(task: F) -> Result<(), ()> {
    crate::kernel::EXECUTOR.read().as_ref().unwrap().spawn(task)
}

/// Print all of the task locations
#[cfg_attr(feature = "backtrace", doors_macros::framed)]
pub async fn print_locations() {
    crate::kernel::EXECUTOR
        .read()
        .as_ref()
        .unwrap()
        .print_locations()
        .await;
}

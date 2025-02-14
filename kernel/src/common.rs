//! Code common to regular kernel and kernel test code

#[path = "executor.rs"]
pub mod executor;
use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
};

use crossbeam::queue::ArrayQueue;
pub use executor::*;

use alloc::sync::Arc;

use crate::kernel;

/// Get the address of the specified variable
pub fn address<T>(v: &T) -> usize {
    v as *const T as usize
}

/// The the address of a slice variable
pub fn slice_address<T>(v: &[T]) -> usize {
    v as *const [T] as *const T as usize
}

/// The trait that allows reading and writing to and from io ports
pub trait IoReadWrite<T> {
    /// Read data from the io port, with the proper size. It is advised that the address be properly aligned for the size of access being performed.
    fn port_read(&mut self) -> T;
    /// Write data to the io port, with the proper size. It is advised that the address be properly aligned for the size of access being performed.
    fn port_write(&mut self, val: T);
}

/// A wrapper that allows for traits to be implemented on an Arc<Mutex<A>>
pub struct LockedArc<A> {
    /// The arc with the contained object
    inner: Arc<Locked<A>>,
}

impl<A> Clone for LockedArc<A> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<A> LockedArc<A> {
    /// Create a new locked arc object.
    pub fn new(inner: A) -> Self {
        Self {
            inner: Arc::new(Locked::new(inner)),
        }
    }

    /// Lock the contained mutex, returning a protected instance of the contained object
    pub fn sync_lock(&self) -> MutexGuard<A> {
        self.inner.sync_lock()
    }

    /// Replace the contents of the protected instance with another instance of the thing
    pub fn replace(&self, r: A) {
        let mut s = self.inner.sync_lock();
        *s = r;
    }
}

/// A wrapper that allows for traits to be implemented on an Arc<Mutex<A>>
pub struct AsyncLockedArc<A> {
    /// The arc with the contained object
    inner: Arc<AsyncLocked<A>>,
}

impl<A> Clone for AsyncLockedArc<A> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<A> AsyncLockedArc<A> {
    /// Create a new locked arc object.
    pub fn new(inner: A) -> Self {
        Self {
            inner: Arc::new(AsyncLocked::new(inner)),
        }
    }

    /// Lock the contained mutex, returning a protected instance of the contained object
    pub fn sync_lock(&self) -> AsyncLockedMutexGuard<A> {
        self.inner.sync_lock()
    }

    /// Lock the contained mutex asynchronously, returning a protected instance of the contained object
    pub async fn lock(&self) -> AsyncLockedMutexGuard<A> {
        self.inner.lock().await
    }

    /// Replace the contents of the protected instance with another instance of the thing
    pub fn sync_replace(&self, r: A) {
        let mut s = self.inner.sync_lock();
        *s = r;
    }

    /// Replace the contents of the protected instance with another instance of the thing
    pub async fn replace(&self, r: A) {
        let mut s = self.inner.lock().await;
        *s = r;
    }
}

/// An async mutex
pub struct AsyncLocked<A: ?Sized> {
    /// The lock
    lock: core::sync::atomic::AtomicBool,
    /// Wakers for the lock
    wakers: Arc<crossbeam::queue::ArrayQueue<futures::task::Waker>>,
    /// The protected data
    data: UnsafeCell<A>,
}

/// The guard for the async mutex
pub struct AsyncLockedMutexGuard<'a, A: ?Sized> {
    /// The lock reference
    lock: &'a core::sync::atomic::AtomicBool,
    /// The unlocked data
    data: *mut A,
    /// The wakers for the mutex
    wakers: Arc<crossbeam::queue::ArrayQueue<futures::task::Waker>>,
}

unsafe impl<A: ?Sized + Send> Sync for AsyncLocked<A> {}
unsafe impl<A: ?Sized + Send> Send for AsyncLocked<A> {}

unsafe impl<A: ?Sized + Sync> Sync for AsyncLockedMutexGuard<'_, A> {}
unsafe impl<A: ?Sized + Send> Send for AsyncLockedMutexGuard<'_, A> {}

/// A struct for a future to lock the mutex
pub struct AsyncLockedMutexGuardFuture<'a, A> {
    /// The inner mutex
    inner: &'a AsyncLocked<A>,
}

impl<'a, A> core::future::Future for AsyncLockedMutexGuardFuture<'a, A> {
    type Output = AsyncLockedMutexGuard<'a, A>;
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        if self
            .inner
            .lock
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            core::task::Poll::Ready(AsyncLockedMutexGuard {
                lock: &self.inner.lock,
                data: unsafe { &mut *self.inner.data.get() },
                wakers: self.inner.wakers.clone(),
            })
        } else {
            self.inner.wakers.push(cx.waker().clone());
            core::task::Poll::Pending
        }
    }
}

impl<A> AsyncLocked<A> {
    /// Construct a new Self
    pub fn new(data: A) -> Self {
        Self {
            lock: core::sync::atomic::AtomicBool::new(false),
            wakers: Arc::new(ArrayQueue::new(32)),
            data: UnsafeCell::new(data),
        }
    }

    /// Synchronously lock the mutex, spinning as necessary
    pub fn sync_lock(&self) -> AsyncLockedMutexGuard<A> {
        loop {
            if self
                .lock
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire)
                .is_ok()
            {
                break AsyncLockedMutexGuard {
                    lock: &self.lock,
                    data: unsafe { &mut *self.data.get() },
                    wakers: self.wakers.clone(),
                };
            }
        }
    }

    /// Lock the mutex, returning the guard
    pub fn lock(&self) -> AsyncLockedMutexGuardFuture<A> {
        AsyncLockedMutexGuardFuture { inner: self }
    }

    /// Replace the contents of the protected instance with another instance of the thing
    pub fn replace(&self, r: A) {
        let mut s = self.sync_lock();
        *s = r;
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for AsyncLockedMutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for AsyncLockedMutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: ?Sized> Deref for AsyncLockedMutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // We know statically that only we are referencing data
        unsafe { &*self.data }
    }
}

impl<T: ?Sized> DerefMut for AsyncLockedMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // We know statically that only we are referencing data
        unsafe { &mut *self.data }
    }
}

impl<T: ?Sized> Drop for AsyncLockedMutexGuard<'_, T> {
    /// The dropping of the MutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
        while let Some(w) = self.wakers.pop() {
            w.wake();
        }
    }
}

/// A wrapper structure that allows for a thing to be wrapped with a mutex.
pub struct Locked<A> {
    /// The contained thing
    inner: spin::Mutex<A>,
}

/// A blank nonsend structure
struct PhantomNonSend {}

impl !Send for PhantomNonSend {}
impl !Sync for PhantomNonSend {}

/// A mutex guard for the Locked structure
pub struct MutexGuard<'a, T> {
    /// The inner mutex
    guard: spin::MutexGuard<'a, T>,
    /// A struct to make the mutex guard non-send
    _dummy: PhantomNonSend,
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

impl<A> Locked<A> {
    /// Create a new protected thing
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    /// Lock the mutex and return a protected instance of the thing
    pub fn sync_lock(&self) -> MutexGuard<A> {
        MutexGuard {
            guard: self.inner.lock(),
            _dummy: PhantomNonSend {},
        }
    }

    /// Replace the contents of the protected instance with another instance of the thing
    pub fn replace(&self, r: A) {
        let mut s = self.inner.lock();
        *s = r;
    }
}

/// A fixed string type that allows for strings of up to 80 characters.
pub type FixedString = arraystring::ArrayString<arraystring::typenum::U80>;

lazy_static::lazy_static! {
    /// The system manger for the kernel
    pub static ref SYSTEM: AsyncLocked<Option<kernel::System>> = AsyncLocked::new(None);
    /// The VGA instance used for x86 kernel printing
    pub static ref VGA: AsyncLockedArc<Option<crate::TextDisplay>> = AsyncLockedArc::new(None);
    /// The VGA2 instance used for x86 kernel printing
    pub static ref VGA2: AsyncLockedArc<Option<crate::TextDisplay>> = AsyncLockedArc::new(None);
}

impl log::Log for AsyncLockedArc<Option<crate::TextDisplay>> {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut s = self.sync_lock();
        use crate::modules::video::TextDisplayTrait;
        if let Some(s) = s.as_mut() {
            s.print_str("LOG RECORD\r\n");
            s.print_str(&doors_macros2::fixed_string_format!("{}", record.level()));
            s.print_str(": ");
            let target = if !record.target().is_empty() {
                record.target()
            } else {
                record.module_path().unwrap_or_default()
            };
            s.print_str(target);
            s.print_str(&doors_macros2::fixed_string_format!("{}", record.args()));
            s.print_str("\r\n");
        } else {
            panic!();
        }
    }

    fn flush(&self) {}
}

impl AsyncLockedArc<Option<crate::TextDisplay>> {
    /// Print a fixed string. This is intended to be used in panic type situations.
    pub fn print_fixed_str(&self, a: FixedString) {
        let mut v = self.sync_lock();
        let vga = v.as_mut();
        if let core::option::Option::Some(vga) = vga {
            use crate::modules::video::TextDisplayTrait;
            vga.print_str(a.as_str());
        }
    }

    /// Print a string. This is intended to be used in panic type situations.
    pub fn print_str(&self, a: &str) {
        let mut v = self.sync_lock();
        let vga = v.as_mut();
        if let core::option::Option::Some(vga) = vga {
            use crate::modules::video::TextDisplayTrait;
            vga.print_str(a);
        }
    }

    /// Print a string reference, asynchronously
    pub async fn print_str_async(&self, a: &str) {
        let mut v = self.lock().await;
        let vga = v.as_mut();
        if let core::option::Option::Some(vga) = vga {
            use crate::modules::video::TextDisplayTrait;
            vga.print_str_async(a).await;
        }
    }
}

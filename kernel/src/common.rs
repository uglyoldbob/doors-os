//! Code common to regular kernel and kernel test code

#[path = "executor.rs"]
pub mod executor;
use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
    sync::atomic::Ordering,
};

pub use executor::*;

use alloc::sync::Arc;

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

/// A wrapper around box that allows for traits to be implemented on a Box
pub struct Box<T> {
    /// The contained object
    inner: alloc::boxed::Box<T>,
}

impl<T: Clone> Clone for Box<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> core::ops::Deref for Box<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> core::ops::DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
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
    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }

    /// Replace the contents of the protected instance with another instance of the thing
    pub fn replace(&self, r: A) {
        let mut s = self.inner.lock();
        *s = r;
    }
}

/// An async mutex
pub struct AsyncLocked<A: ?Sized> {
    /// The lock
    lock: core::sync::atomic::AtomicBool,
    /// The protected data
    data: UnsafeCell<A>,
}

/// The guard for the async mutex
pub struct AsyncLockedMutexGuard<'a, A: ?Sized> {
    /// The lock reference
    lock: &'a core::sync::atomic::AtomicBool,
    /// The unlocked data
    data: *mut A,
}

unsafe impl<A: ?Sized + Send> Sync for AsyncLocked<A> {}
unsafe impl<A: ?Sized + Send> Send for AsyncLocked<A> {}

unsafe impl<A: ?Sized + Sync> Sync for AsyncLockedMutexGuard<'_, A> {}
unsafe impl<A: ?Sized + Send> Send for AsyncLockedMutexGuard<'_, A> {}

impl<A> AsyncLocked<A> {
    /// Construct a new Self
    pub const fn new(data: A) -> Self {
        Self {
            lock: core::sync::atomic::AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Synchronously lock the mutex, spinning as necessary
    pub fn sync_lock(&self) -> AsyncLockedMutexGuard<A> {
        loop {
            if self
                .lock
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                break AsyncLockedMutexGuard {
                    lock: &self.lock,
                    data: unsafe { &mut *self.data.get() },
                };
            }
        }
    }

    /// Lock the mutex, returning the guard
    pub async fn lock(&self) -> AsyncLockedMutexGuard<A> {
        loop {
            if self
                .lock
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                break AsyncLockedMutexGuard {
                    lock: &self.lock,
                    data: unsafe { &mut *self.data.get() },
                };
            }
            executor::Task::yield_now().await;
        }
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
    }
}

/// A wrapper structure that allows for a thing to be wrapped with a mutex.
pub struct Locked<A> {
    /// The contained thing
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    /// Create a new protected thing
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    /// Lock the mutex and return a protected instance of the thing
    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }

    /// Replace the contents of the protected instance with another instance of the thing
    pub fn replace(&self, r: A) {
        let mut s = self.inner.lock();
        *s = r;
    }
}

/// A fixed string type that allows for strings of up to 80 characters.
pub type FixedString = arraystring::ArrayString<arraystring::typenum::U80>;

/// The VGA instance used for x86 kernel printing
pub static VGA: AsyncLocked<Option<crate::TextDisplay>> = AsyncLocked::new(None);

impl AsyncLocked<Option<crate::TextDisplay>> {
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

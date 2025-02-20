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

use alloc::boxed::Box;
use crossbeam::queue::ArrayQueue;
pub use executor::*;
use spin::RwLock;

/// A definition for an Arc. This allows traits to be defined for Arc.
pub struct Arc<T>(alloc::sync::Arc<T>);

impl<T> Deref for Arc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        Arc(self.0.clone())
    }
}

impl<T> Arc<T> {
    /// Creates a new arc
    pub fn new(v: T) -> Self {
        Self(alloc::sync::Arc::new(v))
    }
}

use crate::kernel::{self, SystemTrait};

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
    inner: alloc::sync::Arc<Locked<A>>,
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
            inner: alloc::sync::Arc::new(Locked::new(inner)),
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
    inner: alloc::sync::Arc<AsyncLocked<A>>,
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
            inner: alloc::sync::Arc::new(AsyncLocked::new(inner)),
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
    wakers: alloc::sync::Arc<crossbeam::queue::ArrayQueue<futures::task::Waker>>,
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
    wakers: alloc::sync::Arc<crossbeam::queue::ArrayQueue<futures::task::Waker>>,
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
            wakers: alloc::sync::Arc::new(ArrayQueue::new(32)),
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

/// The system manger for the kernel
pub static SYSTEM: RwLock<kernel::System> = RwLock::new(kernel::NullSystem::new_sys());

lazy_static::lazy_static! {
    /// The VGA instance used for x86 kernel printing
    pub static ref VGA: AsyncLockedArc<Option<crate::kernel::OwnedDevice<crate::TextDisplay>>> = AsyncLockedArc::new(None);
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

impl AsyncLockedArc<Option<crate::kernel::OwnedDevice<crate::TextDisplay>>> {
    /// Stop any async processing for the device if necessary
    pub fn stop_async(&self) {
        let mut v = self.sync_lock();
        let vga = v.as_mut();
        if let core::option::Option::Some(vga) = vga {
            use crate::modules::video::TextDisplayTrait;
            vga.stop_async();
        }
    }

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

    /// Flush all output
    pub fn sync_flush(&self) {
        let mut v = self.sync_lock();
        let vga = v.as_mut();
        if let core::option::Option::Some(vga) = vga {
            use crate::modules::video::TextDisplayTrait;
            vga.flush();
        }
    }
}

/// A wrapper around a structure that should be guarded by disabling interrupts
pub struct IrqGuarded<T> {
    /// The guard value
    value: IrqGuardedInner,
    /// The item being guarded
    inner: AsyncLocked<T>,
}

/// The inner information for an [IrqGuarded] structure
struct IrqGuardedInner {
    /// The irq number used to guard the item
    irqnum: u8,
    /// The unlock function
    unlock: Box<dyn Fn(u8) + Send + Sync>,
    /// The lock function
    lock: Box<dyn Fn(u8) + Send + Sync>,
    /// True when all interrupts should be disabled
    disable_all_interrupts: bool,
}

impl<T> IrqGuarded<T> {
    /// Construct a new self.
    /// #Arguments
    /// * disable_all_interrupts: Set to true when all interrupts should be disabled to protect the data
    /// * inner: The data to protect
    /// * lock: The device specific function to disable the desired interrupt for what is being protected
    /// * unlock: The opposite of lock
    pub fn new(
        irqnum: u8,
        disable_all_interrupts: bool,
        inner: T,
        lock: impl Fn(u8) + Send + Sync + 'static,
        unlock: impl Fn(u8) + Send + Sync + 'static,
    ) -> Self {
        Self {
            value: IrqGuardedInner {
                irqnum,
                unlock: Box::new(unlock),
                lock: Box::new(lock),
                disable_all_interrupts,
            },
            inner: AsyncLocked::new(inner),
        }
    }

    /// Use the inner value from a non-interrupt context
    pub async fn access(&self) -> IrqGuardedUse<T> {
        let sys = crate::SYSTEM.read();
        if self.value.disable_all_interrupts {
            sys.disable_interrupts();
        }
        sys.disable_irq(self.value.irqnum);
        (self.value.lock)(self.value.irqnum);
        IrqGuardedUse {
            r: &self.value,
            val: Some(self.inner.lock().await),
            enable_interrupts: true,
        }
    }

    /// Use the inner value from an interrupt context
    pub fn interrupt_acess(&self) -> IrqGuardedUse<T> {
        IrqGuardedUse {
            r: &self.value,
            val: Some(self.inner.sync_lock()),
            enable_interrupts: false,
        }
    }
}

/// The usable instance of the [IrqGuarded] struct
pub struct IrqGuardedUse<'a, T> {
    /// The reference to the inner struct
    r: &'a IrqGuardedInner,
    /// The unlocked data
    val: Option<AsyncLockedMutexGuard<'a, T>>,
    /// Indicates true when run outside an interrupt context
    enable_interrupts: bool,
}

impl<'a, T> Deref for IrqGuardedUse<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.val.as_ref().unwrap().deref()
    }
}

impl<'a, T> DerefMut for IrqGuardedUse<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.val.as_mut().unwrap().deref_mut()
    }
}

impl<'a, T> Drop for IrqGuardedUse<'a, T> {
    fn drop(&mut self) {
        let sys = crate::SYSTEM.read();
        let a = self.val.take();
        drop(a);
        (self.r.unlock)(self.r.irqnum);
        if self.enable_interrupts {
            sys.enable_irq(self.r.irqnum);
            if self.r.disable_all_interrupts {
                sys.enable_interrupts();
            }
        }
    }
}

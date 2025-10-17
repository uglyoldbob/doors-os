//! Code common to regular kernel and kernel test code

#[path = "executor/mod.rs"]
pub mod executor;
use core::{
    cell::UnsafeCell,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::atomic::{AtomicBool, Ordering},
    task::Waker,
};

/// Do nothing
#[cfg(target_arch = "x86")]
pub fn nop() {
    unsafe { core::arch::asm!("nop;") };
}

/// Do nothing
#[cfg(target_arch = "x86_64")]
pub fn nop() {
    x86_64::instructions::nop();
}

/// Code to idle the system
#[cfg(target_arch = "x86_64")]
pub fn idle() {
    x86_64::instructions::hlt();
}

/// Code to idle the system
#[cfg(target_arch = "x86")]
pub fn idle() {
    unsafe { x86::halt() };
}

/// Code to conditionally idle the system based on a closure
#[cfg(target_arch = "x86_64")]
pub fn idle_if(mut f: impl FnMut() -> bool) {
    crate::SYSTEM.read().disable_interrupts();
    if f() {
        x86_64::instructions::interrupts::enable_and_hlt();
    } else {
        crate::SYSTEM.read().enable_interrupts();
    }
}

/// Code to conditionally idle the system based on a closure
#[cfg(target_arch = "x86")]
pub fn idle_if(mut f: impl FnMut() -> bool) {
    crate::SYSTEM.read().disable_interrupts();
    if f() {
        unsafe { x86::irq::enable() };
        unsafe { x86::halt() };
    } else {
        crate::SYSTEM.read().enable_interrupts();
    }
}

/// This trait is implemented for things safe to use in an interrupt context
pub auto trait Interrupt {}

/// This is a marker type that is safe to use in interrupt contexts
pub struct SafeForInterrupts;

impl Interrupt for SafeForInterrupts {}

/// This is a marker type that is NOT safe to use in interrupt contexts
pub struct NotSafeForInterrupts;

impl !Interrupt for NotSafeForInterrupts {}

use alloc::{boxed::Box, vec::Vec};
use crossbeam::queue::ArrayQueue;
pub use executor::*;
use spin::RwLock;

/// The id for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(usize);

impl Default for TaskId {
    fn default() -> Self {
        static NEXT: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(1);
        Self(NEXT.fetch_add(1, core::sync::atomic::Ordering::Relaxed))
    }
}

impl TaskId {
    /// Get the value
    pub fn value(&self) -> usize {
        self.0
    }
}

impl From<core::num::NonZero<usize>> for TaskId {
    fn from(value: core::num::NonZero<usize>) -> Self {
        Self(value.get())
    }
}

/// A definition for an Arc. This allows traits to be defined for Arc.
pub struct Arc<T>(alloc::sync::Arc<T>);

impl<T> Interrupt for Arc<T> where T: Interrupt {}

impl<T> Interrupt for Box<T> where T: Interrupt + ?Sized {}

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

use crate::kernel::{self, OwnedDevice, SystemTrait};

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
    pub fn sync_lock(&self) -> MutexGuard<'_, A> {
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
    pub fn sync_lock(&self) -> AsyncLockedMutexGuard<'_, A> {
        self.inner.sync_lock()
    }

    /// Lock the contained mutex asynchronously, returning a protected instance of the contained object
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    pub async fn lock(&self) -> AsyncLockedMutexGuard<'_, A> {
        self.inner.lock().await
    }

    /// Replace the contents of the protected instance with another instance of the thing
    pub fn sync_replace(&self, r: A) {
        let mut s = self.inner.sync_lock();
        *s = r;
    }

    /// Replace the contents of the protected instance with another instance of the thing
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    pub async fn replace(&self, r: A) {
        let mut s = self.inner.lock().await;
        *s = r;
    }
}

/// An async mutex
pub struct AsyncLocked<A: ?Sized> {
    /// The lock
    lock: AtomicBool,
    /// Wakers for the lock
    wakers: alloc::sync::Arc<crossbeam::queue::ArrayQueue<futures::task::Waker>>,
    /// The protected data
    data: UnsafeCell<A>,
}

/// The guard for the async mutex
pub struct AsyncLockedMutexGuard<'a, A: ?Sized> {
    /// The lock reference
    lock: &'a AtomicBool,
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
            .compare_exchange(false, true, Ordering::Acquire, Ordering::SeqCst)
            .is_ok()
        {
            core::task::Poll::Ready(AsyncLockedMutexGuard {
                lock: &self.inner.lock,
                data: unsafe { &mut *self.inner.data.get() },
                wakers: self.inner.wakers.clone(),
            })
        } else {
            let _ = self.inner.wakers.push(cx.waker().clone());
            core::task::Poll::Pending
        }
    }
}

impl<A> AsyncLocked<A> {
    /// Construct a new Self
    pub fn new(data: A) -> Self {
        Self {
            lock: AtomicBool::new(false),
            wakers: alloc::sync::Arc::new(ArrayQueue::new(32)),
            data: UnsafeCell::new(data),
        }
    }

    /// Synchronously lock the mutex, spinning as necessary
    pub fn sync_lock(&self) -> AsyncLockedMutexGuard<'_, A> {
        loop {
            if self
                .lock
                .compare_exchange(false, true, Ordering::Acquire, Ordering::SeqCst)
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
    pub fn lock(&self) -> AsyncLockedMutexGuardFuture<'_, A> {
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
        self.lock.store(false, Ordering::SeqCst);
        while let Some(w) = self.wakers.pop() {
            w.wake();
        }
    }
}

/// A wrapper structure that allows for a thing to be wrapped with a mutex.
pub struct Locked<A> {
    /// The lock for protecting the data
    lock: AtomicBool,
    /// The inner data
    inner: UnsafeCell<A>,
}

/// A blank nonsend structure
#[repr(C)]
#[allow(unused)]
struct PhantomNonSend;

impl !Send for PhantomNonSend {}
impl !Sync for PhantomNonSend {}
impl !Interrupt for PhantomNonSend {}

/// A mutex guard for the Locked structure
#[repr(C)]
pub struct MutexGuard<'a, T> {
    /// The inner mutex
    guard: &'a AtomicBool,
    /// The data
    data: *mut T,
}

impl<'a, T> MutexGuard<'a, T> {
    /// Unsafe destroy the lock and return the inner contents
    /// # Safety
    ///
    /// Be sure you know what you are doing!
    pub unsafe fn unsafe_destroy(self) -> *mut T {
        self.guard.store(false, Ordering::Release);
        self.data
    }
}

impl<T> !Send for MutexGuard<'_, T> {}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.guard.store(false, Ordering::Release);
    }
}

unsafe impl<T> Send for Locked<T> {}
unsafe impl<T> Sync for Locked<T> {}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

impl<A> Locked<A> {
    /// Create a new protected thing
    pub const fn new(inner: A) -> Self {
        Locked {
            lock: AtomicBool::new(false),
            inner: UnsafeCell::new(inner),
        }
    }

    /// Lock the mutex and return a protected instance of the thing
    pub fn sync_lock(&self) -> MutexGuard<'_, A> {
        loop {
            if self
                .lock
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                break MutexGuard {
                    guard: &self.lock,
                    data: unsafe { &mut *self.inner.get() },
                };
            }
            doors_macros::todo_item!("Do something with the thread scheduler here");
        }
    }

    /// Replace the contents of the protected instance with another instance of the thing
    pub fn replace(&self, r: A) {
        let mut s = self.sync_lock();
        *s = r;
    }
}

/// A fixed string type that allows for strings of up to 80 characters.
pub type FixedString = arraystring::ArrayString<arraystring::typenum::U80>;

/// The system manger for the kernel
pub static SYSTEM: RwLock<kernel::System> = RwLock::new(kernel::NullSystem::new_sys());

/// The main keyboard
pub static KEYBOARD: RwLock<Option<crate::modules::input::keyboard::Ps2>> = RwLock::new(None);

lazy_static::lazy_static! {
    /// The VGA instance used for x86 kernel printing
    pub static ref VGA: AsyncLockedArc<Option<crate::kernel::OwnedDevice<crate::TextDisplay>>> = AsyncLockedArc::new(None);
}

/// Temporary variable to enable extra kernel logging
pub static DEBUG_PRINT: AtomicBool = AtomicBool::new(false);

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

    /// Use this function for prints that should be together, but require multiple print calls
    pub fn print_with_closure<F>(&self, a: F)
    where
        F: FnOnce(&mut OwnedDevice<crate::TextDisplay>),
    {
        let mut v = self.sync_lock();
        let vga = v.as_mut();
        if let Some(vga) = vga {
            a(vga);
        }
    }

    /// Use this function for prints that should be together, but require multiple print calls
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    pub async fn print_with_async_closure<F>(&self, a: F)
    where
        F: AsyncFnOnce(&mut OwnedDevice<crate::TextDisplay>) -> (),
    {
        let mut v = self.lock().await;
        let vga = v.as_mut();
        if let Some(vga) = vga {
            a(vga).await;
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
            vga.sync_flush();
        }
    }
}

/// This is like [IrqGuarded], but for read only types (they don't need a mutex).
#[derive(Clone)]
pub struct IrqGuardedSimple<T> {
    /// The guard value
    value: IrqGuardedInner,
    /// The item being guarded
    inner: T,
}

impl<T> IrqGuardedSimple<T> {
    /// Construct a new self.
    /// #Arguments
    /// * inner: The data to protect
    /// * common: A reference to the IrqGuardedInner struct already created
    pub fn new(inner: T, common: &IrqGuardedInner) -> Self {
        Self {
            value: common.clone(),
            inner,
        }
    }

    /// Use the inner value from a non-interrupt context
    pub fn access(&self) -> IrqGuardedSimpleUse<'_, T, NotSafeForInterrupts> {
        let sys = crate::SYSTEM.read();
        if self.value.disable_all_interrupts {
            sys.disable_interrupts();
        }
        if self.value.disable_irq {
            for i in self.value.irqnums.iter() {
                sys.disable_irq(i);
            }
        }
        for i in self.value.irqnums.iter() {
            (self.value.lock)(i);
        }
        IrqGuardedSimpleUse {
            r: &self.value,
            val: &self.inner,
            enable_interrupts: true,
            enable_irq: self.value.disable_irq,
            _phantom: PhantomData,
        }
    }

    /// Use the inner value from an interrupt context
    pub fn interrupt_access(&self) -> IrqGuardedSimpleUse<'_, T, SafeForInterrupts> {
        IrqGuardedSimpleUse {
            r: &self.value,
            val: &self.inner,
            enable_interrupts: false,
            enable_irq: false,
            _phantom: PhantomData,
        }
    }
}

/// The usable instance of the [IrqGuardedSimple] struct.
pub struct IrqGuardedSimpleUse<'a, T, U> {
    /// The reference to the inner struct
    r: &'a IrqGuardedInner,
    /// The unlocked data
    val: &'a T,
    /// Indicates true when run outside an interrupt context
    enable_interrupts: bool,
    /// Indicates that irqs should be enabled
    enable_irq: bool,
    /// phantom
    _phantom: PhantomData<U>,
}

impl<'a, T, U> !Send for IrqGuardedSimpleUse<'a, T, U> {}

impl<'a, T, U> Deref for IrqGuardedSimpleUse<'a, T, U> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.val
    }
}

impl<'a, T, U> Drop for IrqGuardedSimpleUse<'a, T, U> {
    fn drop(&mut self) {
        for i in self.r.irqnums.iter() {
            (self.r.unlock)(i);
        }
        if self.enable_interrupts {
            let sys = crate::SYSTEM.read();
            if self.enable_irq {
                for i in self.r.irqnums.iter() {
                    sys.enable_irq(i);
                }
            }
            if self.r.disable_all_interrupts {
                sys.enable_interrupts();
            }
        }
    }
}

/// A wrapper around a structure that should be guarded by disabling interrupts.
/// This is intended to be used on structures that need a mutex in addition to irq protection.
pub struct IrqGuarded<T> {
    /// The guard value
    value: IrqGuardedInner,
    /// The item being guarded
    inner: AsyncLocked<T>,
}

/// Holds a number of irq numbers, for a small number of irqs, this eliminates a memory allocation
#[derive(Clone)]
pub enum IrqNumbers {
    /// No irqs at all
    None,
    /// One irq
    Only1(u8),
    /// Two irq
    Only2([u8; 2]),
    /// Three irq
    Only3([u8; 3]),
    /// Four irq
    Only4([u8; 4]),
    /// Arbitrarily many irq
    Many(Vec<u8>),
}

impl IrqNumbers {
    /// Get an iterator
    pub fn iter(&self) -> IrqNumbersIter<'_> {
        IrqNumbersIter {
            index: 0,
            numbers: self,
        }
    }
}

/// Verifies that a IrqNumbers is the correct size
const _IRQ_NUMBERS_SPACE_CHECKER: [u8; core::mem::size_of::<Vec<u8>>()] =
    [0; core::mem::size_of::<IrqNumbers>()];

/// An iterator over irq numbers
pub struct IrqNumbersIter<'a> {
    /// The index of iteration
    index: u8,
    /// The actual numbers
    numbers: &'a IrqNumbers,
}

impl<'a> Iterator for IrqNumbersIter<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        let val = match self.numbers {
            IrqNumbers::None => None,
            IrqNumbers::Only1(a) => {
                if self.index == 0 {
                    Some(*a)
                } else {
                    None
                }
            }
            IrqNumbers::Only2(a) => {
                let i = self.index;
                a.get(i as usize).copied()
            }
            IrqNumbers::Only3(a) => {
                let i = self.index;
                a.get(i as usize).copied()
            }
            IrqNumbers::Only4(a) => {
                let i = self.index;
                a.get(i as usize).copied()
            }
            IrqNumbers::Many(items) => {
                let i = self.index;
                items.get(i as usize).copied()
            }
        };
        self.index += 1;
        val
    }
}

/// The inner information for an [IrqGuarded] structure
#[derive(Clone)]
pub struct IrqGuardedInner {
    /// The irq number used to guard the item
    irqnums: IrqNumbers,
    /// The unlock function
    unlock: Arc<Box<dyn Fn(u8) + Send + Sync>>,
    /// The lock function
    lock: Arc<Box<dyn Fn(u8) + Send + Sync>>,
    /// True when all interrupts should be disabled
    disable_all_interrupts: bool,
    /// Indicates when the irq should be enable and disabled
    disable_irq: bool,
}

impl IrqGuardedInner {
    /// Construct a new self.
    /// #Arguments
    /// * disable_all_interrupts: Set to true when all interrupts should be disabled to protect the data
    /// * lock: The device specific function to disable the desired interrupt for what is being protected
    /// * unlock: The opposite of lock
    pub fn new(
        irqnums: IrqNumbers,
        disable_all_interrupts: bool,
        disable_irq: bool,
        lock: impl Fn(u8) + Send + Sync + 'static,
        unlock: impl Fn(u8) + Send + Sync + 'static,
    ) -> Self {
        Self {
            irqnums,
            unlock: Arc::new(Box::new(unlock)),
            lock: Arc::new(Box::new(lock)),
            disable_all_interrupts,
            disable_irq,
        }
    }
}

impl Interrupt for IrqGuardedInner {}

impl<T> IrqGuarded<T> {
    /// Construct a new self.
    /// #Arguments
    /// * disable_all_interrupts: Set to true when all interrupts should be disabled to protect the data
    /// * inner: The data to protect
    /// * lock: The device specific function to disable the desired interrupt for what is being protected
    /// * unlock: The opposite of lock
    pub fn new(inner: T, common: &IrqGuardedInner) -> Self {
        Self {
            value: common.clone(),
            inner: AsyncLocked::new(inner),
        }
    }

    /// Return the irq number for the user
    pub fn irqs(&self) -> IrqNumbersIter<'_> {
        self.value.irqnums.iter()
    }

    /// Use the inner value from a non-interrupt context
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    pub async fn access(&self) -> IrqGuardedUse<'_, T, NotSafeForInterrupts> {
        let sys = crate::SYSTEM.read();
        if self.value.disable_all_interrupts {
            sys.disable_interrupts();
        }
        if self.value.disable_irq {
            for i in self.irqs() {
                sys.disable_irq(i);
            }
        }
        for i in self.irqs() {
            (self.value.lock)(i);
        }
        IrqGuardedUse {
            r: &self.value,
            val: Some(self.inner.lock().await),
            enable_interrupts: true,
            enable_irq: self.value.disable_irq,
            _phantom: PhantomData,
        }
    }

    /// Use the inner value from a synchronous non-interrupt context
    pub fn sync_access(&self) -> IrqGuardedUse<'_, T, NotSafeForInterrupts> {
        let sys = crate::SYSTEM.read();
        if self.value.disable_all_interrupts {
            sys.disable_interrupts();
        }
        for i in self.irqs() {
            sys.disable_irq(i);
        }
        for i in self.irqs() {
            (self.value.lock)(i);
        }
        IrqGuardedUse {
            r: &self.value,
            val: Some(self.inner.sync_lock()),
            enable_interrupts: true,
            enable_irq: self.value.disable_irq,
            _phantom: PhantomData,
        }
    }

    /// Use the inner value from an interrupt context
    pub fn interrupt_access(&self) -> IrqGuardedUse<'_, T, SafeForInterrupts> {
        IrqGuardedUse {
            r: &self.value,
            val: Some(self.inner.sync_lock()),
            enable_interrupts: false,
            enable_irq: false,
            _phantom: PhantomData,
        }
    }
}

/// The usable instance of the [IrqGuarded] struct.
pub struct IrqGuardedUse<'a, T, U> {
    /// The reference to the inner struct
    r: &'a IrqGuardedInner,
    /// The unlocked data
    val: Option<AsyncLockedMutexGuard<'a, T>>,
    /// Indicates true when run outside an interrupt context
    enable_interrupts: bool,
    /// Indicates that irqs should be enabled
    enable_irq: bool,
    /// phantom
    _phantom: PhantomData<U>,
}

impl<'a, T, U> Deref for IrqGuardedUse<'a, T, U> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.val.as_ref().unwrap().deref()
    }
}

impl<'a, T, U> DerefMut for IrqGuardedUse<'a, T, U> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.val.as_mut().unwrap().deref_mut()
    }
}

impl<'a, T, U> Drop for IrqGuardedUse<'a, T, U> {
    fn drop(&mut self) {
        let sys = crate::SYSTEM.read();
        let a = self.val.take();
        drop(a);
        for i in self.r.irqnums.iter() {
            (self.r.unlock)(i);
        }
        if self.enable_interrupts {
            if self.enable_irq {
                for i in self.r.irqnums.iter() {
                    sys.enable_irq(i);
                }
            }
            if self.r.disable_all_interrupts {
                sys.enable_interrupts();
            }
        }
    }
}

/// The reader for a one way stream
pub struct IrqStreamReader<T> {
    /// The data queue for the stream
    queue: Arc<IrqGuardedSimple<crossbeam::queue::ArrayQueue<T>>>,
    /// The wakers for the stream
    wakers: Arc<IrqGuardedSimple<crossbeam::queue::ArrayQueue<Waker>>>,
}

impl<T> IrqStreamReader<T> {
    /// Get an element synchronously
    pub fn get_next(&self) -> Option<T> {
        self.queue.access().pop()
    }
}

impl<T> futures::Stream for &IrqStreamReader<T> {
    type Item = T;
    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let a = self.queue.access().pop();
        if let Some(b) = a {
            core::task::Poll::Ready(Some(b))
        } else {
            self.wakers.access().push(cx.waker().clone()).unwrap();
            core::task::Poll::Pending
        }
    }
}

impl<T> futures::Stream for IrqStreamReader<T> {
    type Item = T;
    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let a = self.queue.access().pop();
        if let Some(b) = a {
            core::task::Poll::Ready(Some(b))
        } else {
            self.wakers.access().push(cx.waker().clone()).unwrap();
            core::task::Poll::Pending
        }
    }
}

/// Used for writing data to a OneWayStream
pub struct IrqWriteElement<T> {
    /// The item to write
    stuff: Option<T>,
    /// The data queue for the stream
    queue: Arc<IrqGuardedSimple<crossbeam::queue::ArrayQueue<T>>>,
    /// The wakers for the stream
    wakers: Arc<IrqGuardedSimple<crossbeam::queue::ArrayQueue<Waker>>>,
}

impl<T: Unpin> core::future::Future for IrqWriteElement<T> {
    type Output = ();
    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let q = self.queue.access().is_full();
        if q {
            let _ = self.wakers.access().push(cx.waker().clone());
            return core::task::Poll::Pending;
        } else {
            if let Some(t) = self.stuff.take() {
                self.queue.access().push(t);
                return core::task::Poll::Ready(());
            } else {
                return core::task::Poll::Pending;
            }
        }
    }
}

/// The writer for a one way stream
pub struct IrqStreamWriter<T> {
    /// The data queue for the stream
    queue: Arc<IrqGuardedSimple<crossbeam::queue::ArrayQueue<T>>>,
    /// The wakers for the stream
    wakers: Arc<IrqGuardedSimple<crossbeam::queue::ArrayQueue<Waker>>>,
}

impl<T> Clone for IrqStreamWriter<T> {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
            wakers: self.wakers.clone(),
        }
    }
}

impl<T> IrqStreamWriter<T> {
    /// Add an element to the stream from an interrupt handler
    pub fn push_interrupt(&self, item: T) -> Result<(), ()> {
        self.queue.interrupt_access().push(item).map_err(|_| ())?;
        while let Some(w) = self.wakers.interrupt_access().pop() {
            w.wake();
        }
        Ok(())
    }

    /// Add an element to the stream from a sync context
    pub fn write_sync<'a>(&'a self, val: T) {
        loop {
            let q = self.queue.access().is_full();
            if !q {
                let _ = self.queue.access().push(val);
                break;
            }
        }
        while let Some(w) = self.wakers.interrupt_access().pop() {
            w.wake();
        }
    }

    /// Add an element to the stream from an async context
    pub fn write<'a>(&'a self, val: T) -> IrqWriteElement<T> {
        IrqWriteElement {
            stuff: Some(val),
            queue: self.queue.clone(),
            wakers: self.wakers.clone(),
        }
    }
}

/// Construct a new stream for use in irq handlers
pub fn new_irq_stream<T>(
    inner: &IrqGuardedInner,
    queue_size: usize,
    num_wakers: usize,
) -> (IrqStreamReader<T>, IrqStreamWriter<T>) {
    let queue = Arc::new(IrqGuardedSimple::new(
        crossbeam::queue::ArrayQueue::new(queue_size),
        inner,
    ));
    let wakers = Arc::new(IrqGuardedSimple::new(
        crossbeam::queue::ArrayQueue::new(num_wakers),
        inner,
    ));
    (
        IrqStreamReader {
            queue: queue.clone(),
            wakers: wakers.clone(),
        },
        IrqStreamWriter {
            queue: queue.clone(),
            wakers: wakers.clone(),
        },
    )
}

/// Used for writing data to a OneWayStream
pub struct StreamWriteElement<T> {
    /// The item to write
    stuff: Option<T>,
    /// The data queue for the stream
    queue: Arc<crossbeam::queue::ArrayQueue<T>>,
    /// The wakers for the stream
    wakers: Arc<crossbeam::queue::ArrayQueue<Waker>>,
}

impl<T: Unpin> core::future::Future for StreamWriteElement<T> {
    type Output = ();
    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let q = self.queue.is_full();
        if q {
            let _ = self.wakers.push(cx.waker().clone());
            return core::task::Poll::Pending;
        } else {
            if let Some(t) = self.stuff.take() {
                self.queue.push(t);
                return core::task::Poll::Ready(());
            } else {
                return core::task::Poll::Pending;
            }
        }
    }
}

/// The writer for a one way stream
pub struct StreamWriter<T> {
    /// The data queue for the stream
    queue: Arc<crossbeam::queue::ArrayQueue<T>>,
    /// The wakers for the stream
    wakers: Arc<crossbeam::queue::ArrayQueue<Waker>>,
    /// the marker for no interrupts
    _marker: NotSafeForInterrupts,
}

impl<T> StreamWriter<T> {
    /// Add an element to the stream from an interrupt handler
    pub fn push_interrupt(&self, item: T) -> Result<(), ()> {
        self.queue.push(item).map_err(|_| ())?;
        while let Some(w) = self.wakers.pop() {
            w.wake();
        }
        Ok(())
    }

    /// Add an element to the stream from a sync context
    pub fn write_sync<'a>(&'a self, val: T) {
        loop {
            let q = self.queue.is_full();
            if !q {
                let _ = self.queue.push(val);
                break;
            }
        }
    }

    /// Add an element to the stream from an async context
    pub async fn write<'a>(&'a self, val: T) -> StreamWriteElement<T> {
        StreamWriteElement {
            stuff: Some(val),
            queue: self.queue.clone(),
            wakers: self.wakers.clone(),
        }
    }
}

/// The reader for a one way stream
pub struct StreamReader<T> {
    /// The data queue for the stream
    queue: Arc<crossbeam::queue::ArrayQueue<T>>,
    /// The wakers for the stream
    wakers: Arc<crossbeam::queue::ArrayQueue<Waker>>,
    /// the marker for no interrupts
    _marker: NotSafeForInterrupts,
}

impl<T> StreamReader<T> {
    /// Get an element synchronously
    pub fn get_next(&self) -> Option<T> {
        self.queue.pop()
    }
}

impl<T> futures::Stream for &StreamReader<T> {
    type Item = T;
    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let a = self.queue.pop();
        if let Some(b) = a {
            core::task::Poll::Ready(Some(b))
        } else {
            self.wakers.push(cx.waker().clone()).unwrap();
            core::task::Poll::Pending
        }
    }
}

impl<T> futures::Stream for StreamReader<T> {
    type Item = T;
    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let a = self.queue.pop();
        if let Some(b) = a {
            core::task::Poll::Ready(Some(b))
        } else {
            self.wakers.push(cx.waker().clone()).unwrap();
            core::task::Poll::Pending
        }
    }
}

/// Construct a new stream for general use entirely outside of irq handlers
pub fn new_stream<T>(queue_size: usize, num_wakers: usize) -> (StreamReader<T>, StreamWriter<T>) {
    let queue = Arc::new(crossbeam::queue::ArrayQueue::new(queue_size));
    let wakers = Arc::new(crossbeam::queue::ArrayQueue::new(num_wakers));
    (
        StreamReader {
            queue: queue.clone(),
            wakers: wakers.clone(),
            _marker: NotSafeForInterrupts,
        },
        StreamWriter {
            queue: queue.clone(),
            wakers: wakers.clone(),
            _marker: NotSafeForInterrupts,
        },
    )
}

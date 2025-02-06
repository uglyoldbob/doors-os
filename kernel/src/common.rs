//! Code common to regular kernel and kernel test code

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

/// A struct that manages allocation and deallocation of pci memory
pub struct PciMemory {
    /// The starting address for virtual memory address space
    virt: usize,
    /// The starting address for physical memory address space
    phys: usize,
    /// The size in bytes
    size: usize,
}

impl PciMemory {
    /// Construct a new instance. Should only be used in the memory management code!
    /// # Safety
    /// This function should only be used within memory management code
    /// It constructs a [Self] that is fully specified by the arguments.
    pub unsafe fn build_with(virt: usize, phys: usize, size: usize) -> Self {
        Self { virt, phys, size }
    }

    /// Read a u32 at the specified index (byte based index)
    pub fn read_u32(&self, address: usize) -> u32 {
        let mem = unsafe { core::slice::from_raw_parts(self.virt as *const u8, self.size) };
        let a: &u8 = &mem[address];
        let b: *const u8 = a as *const u8;
        let c: &u32 = unsafe { &*(b as *const u32) };
        unsafe { core::ptr::read_volatile(c) }
    }

    /// Write a u32 at the specified index (byte based index), with the specified value
    pub fn write_u32(&mut self, address: usize, val: u32) {
        let mem = unsafe { core::slice::from_raw_parts_mut(self.virt as *mut u8, self.size) };
        let a: &mut u8 = &mut mem[address];
        let b: *mut u8 = a as *mut u8;
        let c: &mut u32 = unsafe { &mut *(b as *mut u32) };
        unsafe { core::ptr::write_volatile(c, val) };
    }

    /// Get the size of the memory area in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the starting physical address for the region
    pub fn phys(&self) -> usize {
        self.phys
    }

    /// Get the starting virtual address for the region
    pub fn virt(&self) -> usize {
        self.virt
    }
}

/// A structure that generically maps dma memory over a type.
pub struct DmaMemory<T> {
    /// The starting address for virtual memory address space
    virt: usize,
    /// The starting address for physical memory address space
    phys: usize,
    /// The size in bytes
    size: usize,
    /// The data (in virtual memory space)
    data: alloc::boxed::Box<T>,
}

impl<T> DmaMemory<T> {
    /// Construct a new instance. Should only be used in the memory management code!
    /// # Safety
    /// This function should only be used within memory management code
    /// It constructs a [Self] that is fully specified by the arguments.
    pub unsafe fn build_with(
        virt: usize,
        phys: usize,
        size: usize,
        data: alloc::boxed::Box<T>,
    ) -> Self {
        Self {
            virt,
            phys,
            size,
            data,
        }
    }

    /// Get the size of the memory area in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the starting physical address for the region
    pub fn phys(&self) -> usize {
        self.phys
    }

    /// Get the starting virtual address for the region
    pub fn virt(&self) -> usize {
        self.virt
    }
}

impl<T> core::ops::Deref for DmaMemory<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> core::ops::DerefMut for DmaMemory<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

/// Used to store an array of items for dma
pub struct DmaMemorySlice<T> {
    /// The starting address for virtual memory address space
    virt: usize,
    /// The starting address for physical memory address space
    phys: usize,
    /// The size in bytes
    size: usize,
    /// The data (in virtual memory space)
    data: alloc::vec::Vec<T>,
}

impl<T> DmaMemorySlice<T> {
    /// Construct a new instance. Should only be used in the memory management code!
    /// # Safety
    /// This function should only be used within memory management code
    /// It constructs a [Self] that is fully specified by the arguments.
    pub unsafe fn build_with(
        virt: usize,
        phys: usize,
        size: usize,
        data: alloc::vec::Vec<T>,
    ) -> Self {
        Self {
            virt,
            phys,
            size,
            data,
        }
    }

    /// Get the size of the memory area in bytes
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get the starting physical address for the region
    pub fn phys(&self) -> usize {
        self.phys
    }

    /// Get the starting virtual address for the region
    pub fn virt(&self) -> usize {
        self.virt
    }
}

impl<T> core::ops::Deref for DmaMemorySlice<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> core::ops::DerefMut for DmaMemorySlice<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
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
pub static VGA: Locked<Option<crate::TextDisplay>> = Locked::new(None);

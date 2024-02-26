//! This is the kernel for the doors operating system. It is written in rust and pieces of it (as required) are written in assembly.

#![no_std]
#![no_main]
#![deny(missing_docs)]
#![deny(clippy::missing_docs_in_private_items)]
#![feature(allocator_api)]
#![feature(strict_provenance)]
#![feature(const_mut_refs)]
#![feature(alloc_error_handler)]

extern crate alloc;

pub mod boot;
pub mod kernel;
pub mod modules;

use alloc::sync::Arc;
use modules::video::TextDisplay;
use modules::video::TextDisplayTrait;

use crate::modules::gpio::GpioPinTrait;
use crate::modules::timer::TimerTrait;
use crate::modules::video::mipi_dsi::MipiDsiProvider;
use crate::modules::video::mipi_dsi::MipiDsiTrait;

/// A fixed string type that allows for strings of up to 80 characters.
pub type FixedString = arraystring::ArrayString<arraystring::typenum::U80>;

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

/// This creates the multiboot2 signature that allows the kernel to be booted by a multiboot compliant bootloader such as grub.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
#[link_section = ".multiboot"]
#[used]
static MULTIBOOT_HEADER: boot::multiboot::Multiboot = boot::multiboot::Multiboot::new();

/// The VGA instance used for x86 kernel printing
static VGA: spin::Mutex<Option<TextDisplay>> = spin::Mutex::new(None);

/// Used to debug some stuff in the kernel
pub static DEBUG_STUFF: Locked<[u32; 82]> = Locked::new([0; 82]);

struct ColorCycler {
    color: u16,
    num_bits: u8,
    shift: u8,
}

impl ColorCycler {
    fn new() -> Self {
        Self {
            color: 1,
            num_bits: 1,
            shift: 0,
        }
    }

    fn get_color(&self) -> u16 {
        self.color
    }

    fn advance(&mut self) {
        if (self.num_bits + self.shift) < 15 {
            self.shift += 1;
        } else {
            self.shift = 0;
            if self.num_bits < 14 {
                self.num_bits += 1;
            } else {
                self.num_bits = 1;
            }
        }

        let ones = 0xffff >> (16 - self.num_bits);
        let val = ones << self.shift;
        self.color = val;
    }
}

fn main() -> ! {
    {
        use crate::modules::gpio::GpioTrait;

        let mut gpio = crate::kernel::GPIO.lock();
        let mg = gpio.module(0);

        let mj = gpio.module(9);
        drop(gpio);
        let gpioa = mg.lock();
        let j = mj.lock();

        let mut mco_pin = gpioa.get_pin(8).unwrap();
        mco_pin.set_alternate(0);
        mco_pin.set_speed(3);

        let mut count = 0;

        let mut led1 = gpioa.get_pin(12).unwrap();
        let mut led2 = j.get_pin(5).unwrap();
        let mut led3 = j.get_pin(13).unwrap();

        doors_macros2::kernel_print!("DoorsOs Booting now\r\n");

        let mut timers = crate::kernel::TIMERS.lock();
        let tp = timers.module(0);
        drop(timers);
        let mut tpl = tp.lock();
        let timer = tpl.get_timer(0);
        drop(tpl);

        led1.set_output();
        led2.set_output();
        led3.set_output();

        let testing2 =
            unsafe { core::slice::from_raw_parts_mut(0xc000_0000 as *mut u16, 800 * 480) };

        let mut color = ColorCycler::new();
        let mut led = false;

        loop {
            led1.write_output(led);
            led2.write_output(led);
            led3.write_output(led);

            count += 1;
            if count > 10 {
                count = 0;
            }
            for e in testing2.iter_mut() {
                *e = color.get_color();
            }
            for i in 0..800 {
                testing2[i * 480 + 32] = 0;
            }
            color.advance();
            led = !led;

            if let Ok(timer) = &timer {
                crate::modules::timer::TimerInstanceTrait::delay_ms(timer, 250);
            }
        }
    }
}

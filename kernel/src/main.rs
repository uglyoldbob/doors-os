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
use alloc::vec::Vec;
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

fn main() -> ! {
    doors_macros2::kernel_print!("I am groot\r\n");
    {
        use crate::modules::gpio::GpioTrait;
        use crate::modules::serial::SerialTrait;
        use crate::modules::timer::TimerInstanceTrait;

        let mut serials = crate::kernel::SERIAL.lock();
        let serial = serials.module(0);
        drop(serials);
        let s = serial.lock();
        s.setup(115200);
        drop(s);
        let mut v = VGA.lock();
        v.replace(TextDisplay::SerialDisplay(
            crate::modules::video::VideoOverSerial::new(serial),
        ));
        drop(v);

        let mut gpio = crate::kernel::GPIO.lock();
        let mg = gpio.module(0);

        let mj = gpio.module(9);
        let mi = gpio.module(8);
        drop(gpio);
        let mut gpioa = mg.lock();

        let mut gpioi = mi.lock();
        let mut j = mj.lock();
        gpioa.reset(false);
        j.reset(false);

        //set the pin for the mco1 clock output
        gpioa.set_alternate(8, 0);
        //set the pins for the uart hardware
        gpioa.set_alternate(9, 7);
        gpioa.set_alternate(10, 7);
        //enable high speed output for the clock output
        gpioa.set_speed(8, 3);

        gpioa.set_output(12);
        j.set_output(5);
        j.set_output(13);
        let mut count = 0;

        gpioi.reset(false);
        let lcd_backlight = gpioi.get_pin(14).unwrap();
        let mut led1 = gpioa.get_pin(12).unwrap();
        let mut led2 = j.get_pin(5).unwrap();
        let mut led3 = j.get_pin(13).unwrap();

        let lcd_reset = j.get_pin(15).unwrap();

        doors_macros2::kernel_print!("DoorsOs Booting now\r\n");

        let dsi_config = crate::modules::video::mipi_dsi::MipiDsiConfig {
            link_speed: 500_000_000,
            num_lanes: 2,
            vcid: 0,
        };

        let mut displays = crate::kernel::DISPLAYS.lock();
        let dsi = displays.module(0);
        let dsi = dsi.lock();
        let panel = Some(
            crate::modules::video::mipi_dsi::DsiPanel::OrisetechOtm8009a(LockedArc::new(
                crate::modules::video::mipi_dsi::OrisetechOtm8009a::new(
                    lcd_reset,
                    Some(lcd_backlight),
                ),
            )),
        );
        dsi.enable(&dsi_config, panel);
        drop(dsi);
        drop(displays);

        let mut timers = crate::kernel::TIMERS.lock();
        let tp = timers.module(0);
        drop(timers);
        let mut tpl = tp.lock();
        let timer = tpl.get_timer(0);
        drop(tpl);

        led1.set_output();
        led2.set_output();
        led2.set_output();

        let mut testing: Vec<u32> = Vec::with_capacity(1024);
        for _ in 0..1024 {
            testing.push(0);
        }

        let advance_val = |a: u32| {
            if (a & 1) != 0 {
                0x3f003f<<5
            }
            else if (a & 0x80) != 0 {
                0x1f001f<<11
            }
            else {
                0x1f001f
            }
        };
        let mut color : u32 = 0x1F001F;

        loop {
            led1.write_output(true);
            led2.write_output(true);
            led3.write_output(true);

            count += 1;
            if count > 10 {
                count = 0;
            }
            for e in &mut testing {
                *e = color;
            }
            color = advance_val(color);

            if let Ok(timer) = &timer {
                crate::modules::timer::TimerInstanceTrait::delay_ms(timer, 100);
            }

            //doors_macros2::kernel_print!("I am groot {}\r\n", count);

            led1.write_output(false);
            led2.write_output(false);
            led3.write_output(false);
            for e in &mut testing {
                *e = color;
            }
            color = advance_val(color);

            if let Ok(timer) = &timer {
                crate::modules::timer::TimerInstanceTrait::delay_ms(timer, 100);
            }
        }
    }
}

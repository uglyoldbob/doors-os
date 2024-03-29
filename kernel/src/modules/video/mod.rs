//! Video related kernel modules

use alloc::vec::Vec;

use crate::LockedArc;

use super::serial::SerialTrait;

pub mod text;
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod vga;

pub mod mipi_dsi;

/// A simple memory based framebuffer
pub struct SimpleRamFramebuffer {
    /// The actual contents of the framebuffer
    buffer: Vec<u8>,
}

impl SimpleRamFramebuffer {
    ///Make a ram framebuffer of the specified size
    pub fn new(size: usize) -> Self {
        Self {
            buffer: alloc::vec![0; size],
        }
    }
}

impl FramebufferTrait for SimpleRamFramebuffer {
    unsafe fn address(&self) -> usize {
        self.buffer.as_ptr() as usize
    }
}

/// The trait for all framebuffer devices
#[enum_dispatch::enum_dispatch]
pub trait FramebufferTrait {
    /// Return the address of the framebuffer
    unsafe fn address(&self) -> usize;
}

/// A framebuffer for the kernel
#[enum_dispatch::enum_dispatch(FramebufferTrait)]
pub enum Framebuffer {
    /// A framebuffer that lives in plain memory
    SimpleRam(SimpleRamFramebuffer),
}

/// The various types of graphics displays that can exist
pub enum Display {
    /// A framebuffer based display
    Framebuffer(Framebuffer),
}

/// This trait is used for text only video output hardware
#[enum_dispatch::enum_dispatch]
pub trait TextDisplayTrait: Sync + Send {
    /// Write a single character to the video hardware
    fn print_char(&mut self, d: char);

    /// Write an array of characters to the video hardware
    fn print_str(&mut self, d: &str) {
        for c in d.chars() {
            self.print_char(c);
        }
    }

    /// Repeatedly prints a given character a certain number of times
    fn print_repeat_letter(&mut self, d: char, n: u8) {
        for _ in 0..=n {
            self.print_char(d);
        }
    }
}

/// An enumeration of all the types of text display options
#[enum_dispatch::enum_dispatch(TextDisplayTrait)]
pub enum TextDisplay {
    /// A serial port used for displaying text
    SerialDisplay(VideoOverSerial),
    /// X86 vga hardware operated in text mode
    X86VgaTextMode(text::X86VgaTextMode),
}

/// Enables sending video text over a serial port
pub struct VideoOverSerial {
    port: LockedArc<super::serial::Serial>,
}

impl VideoOverSerial {
    /// Build a new video over serial device
    pub fn new(s: LockedArc<super::serial::Serial>) -> Self {
        Self { port: s }
    }
}

impl TextDisplayTrait for VideoOverSerial {
    fn print_char(&mut self, d: char) {
        let port = self.port.lock();
        let mut c = [0; 4];
        let s = d.encode_utf8(&mut c);
        port.sync_transmit_str(s);
    }

    fn print_str(&mut self, d: &str) {
        let port = self.port.lock();
        port.sync_transmit_str(d);
    }
}

/// Represents a screen resolution
#[derive(Clone)]
pub struct ScreenResolution {
    /// The number of active pixels across the screen
    pub width: u16,
    /// The number of active rows
    pub height: u16,
    /// The width of the hsync pulse in pixels
    pub hsync: u16,
    /// The height of the vsync pulse in rows
    pub vsync: u16,
    /// The width of the horizontal back porch, in pixels
    pub h_b_porch: u16,
    /// The width of the horizontal front porch, in pixels
    pub h_f_porch: u16,
    /// The height of the vertical back porch, in rows
    pub v_b_porch: u16,
    /// The height of the vertical front porch, in rows
    pub v_f_porch: u16,
}

/// Type used for the pc vga text mode output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct VgaChar {
    /// The ascii character to print to the screen
    ascii: u8,
    /// The foreground and background color for the character
    color: u8,
}

impl core::ops::Deref for VgaChar {
    type Target = Self;
    fn deref(&self) -> &Self::Target {
        self
    }
}

impl core::ops::DerefMut for VgaChar {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}

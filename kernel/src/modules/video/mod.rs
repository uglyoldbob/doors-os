//! Video related kernel modules

use alloc::vec::Vec;
use fonts::VariableWidthFont;
use pixels::FullColor;

use crate::{AsyncLockedArc, LockedArc};

use super::serial::SerialTrait;

pub mod fonts;
pub mod pixels;

pub mod text;
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod vga;

pub mod mipi_dsi;

use lazy_static::lazy_static;

/// The data required to render a single font character
pub struct FontData {
    /// the width of the character in pixels
    width: u8,
    /// the height of the character in pixels
    height: u8,
    /// Represents a parameter of how to print the character. TODO
    left: i8,
    /// Represents a parameter of how to print the character. TODO
    top: i8,
    /// The font data for a single character
    data: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/fontmap.rs"));

lazy_static! {
    /// The general font to use for terminals
    pub static ref MAIN_FONT: Font<pixels::FullColor<u32>> = Font::VariableWidth(VariableWidthFont::new(&FONTMAP));
    /// The general palette font for terminals
    pub static ref MAIN_FONT_PALETTE: Font<pixels::Palette<u8>> = Font::VariableWidth(VariableWidthFont::new(&FONTMAP));
}

/// Represents a flat representation of a frame buffer
pub struct OpaqueFrameBuffer<'a, P> {
    /// the buffer data
    _buffer: &'a [u8],
    /// width in pixels
    _width: u16,
    /// height in pixels
    _height: u16,
    /// phantom data that the framebuffer actually stores
    pixel: core::marker::PhantomData<P>,
}

/// Represents a rectangular frame buffer
pub struct PlainFrameBuffer<'a, P> {
    /// The buffer reference
    _buffer: &'a [P],
    /// The width in pixels
    _width: u16,
    /// The height in pixels
    _height: u16,
}

/// A simple memory based framebuffer
pub struct SimpleRamFramebuffer {
    /// The actual contents of the framebuffer
    buffer: Vec<u8>,
    /// The width fo the framebuffer in pixels
    _width: u16,
    /// The height of the framebuffer in pixels
    height: u16,
}

impl SimpleRamFramebuffer {
    ///Make a ram framebuffer of the specified size
    pub fn new(h: u16, w: u16, size: usize) -> Self {
        let mut s = Self {
            buffer: alloc::vec![0; size],
            _width: w,
            height: h,
        };
        for a in &mut s.buffer {
            *a = 0;
        }
        s
    }
}

impl FramebufferTrait<pixels::Palette<u8>> for SimpleRamFramebuffer {
    fn write_plain(&mut self, _x: u16, _y: u16, _fb: PlainFrameBuffer<'_, pixels::Palette<u8>>) {
        todo!()
    }

    fn write_opaque(&mut self, _x: u16, _y: u16, _ob: OpaqueFrameBuffer<'_, pixels::Palette<u8>>) {
        todo!()
    }

    fn write_pixel(&mut self, _x: u16, _y: u16, _p: pixels::Palette<u8>) {
        todo!()
    }
}

impl FramebufferTrait<pixels::FullColor<u32>> for SimpleRamFramebuffer {
    fn write_plain(&mut self, _x: u16, _y: u16, _fb: PlainFrameBuffer<'_, pixels::FullColor<u32>>) {
        todo!()
    }

    fn write_opaque(
        &mut self,
        _x: u16,
        _y: u16,
        _ob: OpaqueFrameBuffer<'_, pixels::FullColor<u32>>,
    ) {
        todo!()
    }

    fn write_pixel(&mut self, x: u16, y: u16, p: pixels::FullColor<u32>) {
        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
            "Pixel offset {}\r\n",
            (self.height as usize * x as usize + y as usize)
        ));
        crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
            "Pixel offset {}\r\n",
            (2 * (self.height as usize * x as usize + y as usize)) + 1
        ));
        //doors_macros2::kernel_print!("Pixel offset {}\r\n", (2 * (self.height as usize * x as usize + y as usize))+2);
        self.buffer[self.height as usize * x as usize + y as usize] = (p.pixel & 0xff) as u8;
        self.buffer[(2 * (self.height as usize * x as usize + y as usize)) + 1] =
            ((p.pixel >> 8) & 0xff) as u8;
        //self.buffer[(3 * (self.height as usize * x as usize + y as usize))+2] = (p.pixel>>16 & 0xff) as u8;
        //self.buffer[(3 * (self.width as usize * x as usize + y as usize))+3] = (p.pixel>>24 & 0xff) as u8;
    }
}

/// The trait for all framebuffer devices
#[enum_dispatch::enum_dispatch]
pub trait FramebufferTrait<P> {
    /// Write a plain frame buffer to the device
    /// #Arguments
    /// * x - The x coordinate of the top left corner to draw
    /// * y - The y coordinate of the top left corner to draw
    /// * fb - The framebuffer to write to the device
    fn write_plain(&mut self, x: u16, y: u16, fb: PlainFrameBuffer<'_, P>);
    /// Write an opaque frame buffer to the device
    /// #Arguments
    /// * x - The x coordinate of the top left corner to draw
    /// * y - The y coordinate of the top left corner to draw
    /// * fb - The framebuffer to write to the device
    fn write_opaque(&mut self, x: u16, y: u16, ob: OpaqueFrameBuffer<'_, P>);
    /// Write a single pixel to the device
    /// #Arguments
    /// * x - The x coordinate to draw
    /// * y - The y coordinate to draw
    /// * p - The pixel to draw
    fn write_pixel(&mut self, x: u16, y: u16, p: P);
}

/// A framebuffer for the kernel
pub enum Framebuffer {
    /// A framebuffer that lives in plain memory
    SimpleRam(SimpleRamFramebuffer),
    #[cfg(kernel_machine = "pc64")]
    /// x86 vga hardware
    VgaHardware(vga::X86VgaMode),
}

impl FramebufferTrait<pixels::Palette<u8>> for Framebuffer {
    fn write_plain(&mut self, x: u16, y: u16, fb: PlainFrameBuffer<'_, pixels::Palette<u8>>) {
        match self {
            Framebuffer::SimpleRam(f) => f.write_plain(x, y, fb),
            #[cfg(kernel_machine = "pc64")]
            Framebuffer::VgaHardware(f) => f.write_plain(x, y, fb),
        }
    }

    fn write_opaque(&mut self, _x: u16, _y: u16, _ob: OpaqueFrameBuffer<'_, pixels::Palette<u8>>) {
        todo!()
    }

    fn write_pixel(&mut self, x: u16, y: u16, p: pixels::Palette<u8>) {
        match self {
            Framebuffer::SimpleRam(f) => f.write_pixel(x, y, p),
            #[cfg(kernel_machine = "pc64")]
            Framebuffer::VgaHardware(f) => f.write_pixel(x, y, p),
        }
    }
}

impl FramebufferTrait<pixels::FullColor<u32>> for Framebuffer {
    fn write_plain(&mut self, x: u16, y: u16, fb: PlainFrameBuffer<'_, pixels::FullColor<u32>>) {
        match self {
            Framebuffer::SimpleRam(f) => f.write_plain(x, y, fb),
            #[cfg(kernel_machine = "pc64")]
            Framebuffer::VgaHardware(_f) => todo!(),
        }
    }

    fn write_opaque(&mut self, x: u16, y: u16, ob: OpaqueFrameBuffer<'_, pixels::FullColor<u32>>) {
        match self {
            Framebuffer::SimpleRam(f) => f.write_opaque(x, y, ob),
            #[cfg(kernel_machine = "pc64")]
            Framebuffer::VgaHardware(_f) => todo!(),
        }
    }

    fn write_pixel(&mut self, x: u16, y: u16, p: pixels::FullColor<u32>) {
        match self {
            Framebuffer::SimpleRam(f) => f.write_pixel(x, y, p),
            #[cfg(kernel_machine = "pc64")]
            Framebuffer::VgaHardware(_f) => todo!(),
        }
    }
}

impl Framebuffer {
    /// Get an iterator over every byte in the framebuffer
    pub fn iter_bytes(&mut self) -> core::slice::IterMut<u8> {
        match self {
            Framebuffer::SimpleRam(simple_ram_framebuffer) => {
                simple_ram_framebuffer.buffer.iter_mut()
            }
            #[cfg(kernel_machine = "pc64")]
            Framebuffer::VgaHardware(_vga) => todo!(),
        }
    }

    /// Debug print an important address for the display
    pub fn print_address(&self) {
        match self {
            Framebuffer::SimpleRam(fb) => crate::VGA.print_fixed_str(
                doors_macros2::fixed_string_format!("FB IS AT {:p}\r\n", &fb.buffer[0]),
            ),
            #[cfg(kernel_machine = "pc64")]
            Framebuffer::VgaHardware(_vga) => todo!(),
        }
    }

    /// Create a text console with the framebuffer
    pub fn make_console(self, font: &'static Font<pixels::FullColor<u32>>) -> TextDisplay {
        match self {
            Framebuffer::SimpleRam(fb) => TextDisplay::FramebufferTextFull(
                FramebufferTextMode::new(Framebuffer::SimpleRam(fb), Some(font)),
            ),
            #[cfg(kernel_machine = "pc64")]
            Framebuffer::VgaHardware(_vga) => todo!(),
        }
    }

    /// Create a paletted text console with the framebuffer
    pub fn make_console_palette(self, font: &'static Font<pixels::Palette<u8>>) -> TextDisplay {
        match self {
            Framebuffer::SimpleRam(fb) => TextDisplay::FramebufferTextPalette(
                FramebufferTextMode::new(Framebuffer::SimpleRam(fb), Some(font)),
            ),
            #[cfg(kernel_machine = "pc64")]
            Framebuffer::VgaHardware(vga) => {
                TextDisplay::X86VgaGraphicsMode(vga::X86VgaWithFont::new(vga, font))
            }
        }
    }
}

/// The various types of graphics displays that can exist
pub enum Display {
    /// A framebuffer based display
    Framebuffer(Framebuffer),
}

impl Display {
    /// Try to get a framebuffer, if applicable for the display
    pub fn try_get_pixel_buffer(&mut self) -> Option<&mut Framebuffer> {
        match self {
            Display::Framebuffer(framebuffer) => Some(framebuffer),
        }
    }

    /// Make a console out of the display
    pub fn make_console(self) -> TextDisplay {
        match self {
            Display::Framebuffer(fb) => {
                let f = &*MAIN_FONT;
                fb.make_console(f)
            }
        }
    }
}

/// The trait to get a tiny framebuffer for each font character
#[enum_dispatch::enum_dispatch]
pub trait FontTrait<P>: Sync + Send {
    /// Perform a lookup to get the graphics for a given character
    fn lookup_symbol(&self, c: char) -> Option<&FontData>;
    /// The height of the font in pixels
    fn height(&self) -> u16;
    /// All valid symbols for the font
    fn symbols(&self) -> alloc::collections::btree_map::Iter<char, FontData>;
}

/// The fonts that can exist
pub enum Font<P> {
    /// A fixed width font
    FixedWidth(fonts::FixedWidthFont<P>),
    /// A variable width font
    VariableWidth(fonts::VariableWidthFont<P>),
}

impl<P: Send + Sync> FontTrait<P> for Font<P> {
    fn lookup_symbol(&self, c: char) -> Option<&FontData> {
        match self {
            Font::FixedWidth(f) => f.lookup_symbol(c),
            Font::VariableWidth(f) => f.lookup_symbol(c),
        }
    }

    fn height(&self) -> u16 {
        match self {
            Font::FixedWidth(f) => f.height(),
            Font::VariableWidth(f) => f.height(),
        }
    }

    fn symbols(&self) -> alloc::collections::btree_map::Iter<char, FontData> {
        match self {
            Font::FixedWidth(f) => f.symbols(),
            Font::VariableWidth(f) => f.symbols(),
        }
    }
}

/// This trait is used for text only video output hardware
#[enum_dispatch::enum_dispatch]
pub trait TextDisplayTrait: Sync + Send {
    /// Stop all interrupt and async operations
    fn stop_async(&mut self);

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

    /// Asynchronously print a character
    async fn print_char_async(&mut self, d: char);

    /// Asynchronously print a string
    async fn print_str_async(&mut self, d: &str) {
        for c in d.chars() {
            self.print_char_async(c).await;
        }
    }

    /// Asynchrouously flush all data
    async fn flush(&mut self);

    /// Synchronously flush all data
    fn sync_flush(&mut self);
}

/// Draws text onto a framebuffer
pub struct FramebufferTextMode<P: 'static> {
    /// The framebuffer that font is rendered to
    fb: Framebuffer,
    /// The font reference to use for printing text to the frambuffer
    font: &'static Font<P>,
    /// The horizontal position of the cursor in pixels
    cursor_x: u8,
    /// The vertical position of the cursor in pixels
    cursor_y: u8,
}

impl<P> FramebufferTextMode<P>
where
    P: Sync + Send,
{
    ///Construct a new Self
    pub fn new(fb: Framebuffer, font: Option<&'static Font<P>>) -> Self {
        let font = match font {
            Some(f) => f,
            None => todo!(),
        };
        Self {
            fb,
            font,
            cursor_x: 0,
            cursor_y: 0,
        }
    }
}

impl TextDisplayTrait for FramebufferTextMode<pixels::Palette<u8>> {
    fn print_char(&mut self, _d: char) {
        for x in 0..50 {
            for y in 0..10 {
                let pixel: crate::modules::video::pixels::Palette<u8> =
                    crate::modules::video::pixels::Palette::new(
                        0x11,
                        crate::modules::video::vga::DEFAULT_PALETTE,
                    );
                self.fb.write_pixel(x, y, pixel);
            }
        }
    }

    async fn print_char_async(&mut self, d: char) {
        self.print_char(d);
    }

    async fn flush(&mut self) {}

    fn sync_flush(&mut self) {}

    fn stop_async(&mut self) {}
}

impl<P> TextDisplayTrait for FramebufferTextMode<pixels::FullColor<P>>
where
    P: Sync + Send,
{
    fn print_char(&mut self, d: char) {
        let a = FontTrait::lookup_symbol(self.font, d);
        if let Some(_a) = a {
            for i in 0..10 {
                for j in 0..10 {
                    let p = 0xffffffff;
                    let p = FullColor::<u32>::new(p);
                    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                        "Print pixel {} {}\r\n",
                        self.cursor_x as u16 + i,
                        self.cursor_y as u16 + j
                    ));
                    self.fb
                        .write_pixel(self.cursor_x as u16 + i, self.cursor_y as u16 + j, p);
                }
            }
            crate::VGA.print_str("Done printing char\r\n");
            self.cursor_x += 5;
        }
        todo!();
    }

    async fn print_char_async(&mut self, d: char) {
        self.print_char(d);
    }

    async fn flush(&mut self) {}

    fn sync_flush(&mut self) {}

    fn stop_async(&mut self) {}
}

/// An enumeration of all the types of text display options
#[enum_dispatch::enum_dispatch(TextDisplayTrait)]
pub enum TextDisplay {
    /// A serial port used for displaying text
    SerialDisplay(VideoOverSerial),
    /// X86 vga hardware operated in text mode
    X86VgaTextMode(text::X86VgaTextMode),
    /// X86 vga hardware operated in graphics mode
    X86VgaGraphicsMode(vga::X86VgaWithFont<pixels::Palette<u8>>),
    /// Paletted framebuffer
    FramebufferTextPalette(FramebufferTextMode<pixels::Palette<u8>>),
    /// Full color framebuffer
    FramebufferTextFull(FramebufferTextMode<pixels::FullColor<u32>>),
}

/// Enables sending video text over a serial port
pub struct VideoOverSerial {
    /// The serial port to send data over
    port: super::serial::Serial,
}

impl VideoOverSerial {
    /// Build a new video over serial device
    pub fn new(s: super::serial::Serial) -> Self {
        Self { port: s }
    }
}

impl TextDisplayTrait for VideoOverSerial {
    fn print_char(&mut self, d: char) {
        let mut c = [0; 4];
        let s = d.encode_utf8(&mut c);
        self.port.sync_transmit_str(s);
    }

    fn print_str(&mut self, d: &str) {
        self.port.sync_transmit_str(d);
    }

    async fn print_char_async(&mut self, d: char) {
        let mut c = [0; 4];
        let s = d.encode_utf8(&mut c);
        self.port.transmit_str(s).await;
    }

    async fn print_str_async(&mut self, d: &str) {
        self.port.transmit_str(d).await;
    }

    async fn flush(&mut self) {
        self.port.flush().await;
    }

    fn sync_flush(&mut self) {
        self.port.sync_flush();
    }

    fn stop_async(&mut self) {
        self.port.stop_async();
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

/// prints out a user friendly hex dump of the specified data
pub fn hex_dump(data: &[u8], print_address: bool, print_ascii: bool) {
    let len = data.len();
    let mut addr_len = 1;
    let mut len_calc = len;
    loop {
        if len_calc > 15 {
            addr_len += 1;
            len_calc /= 16;
        } else {
            break;
        }
    }
    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
        "ADDRESS IS {:p}, size {:x}\r\n",
        data,
        data.len()
    ));
    for (i, b) in data.chunks(16).enumerate() {
        if print_address {
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!(
                "{:0>addr_len$x}: ",
                i * 16
            ));
        }
        for d in b {
            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!("{:02x} ", d));
        }
        if print_ascii {
            for _ in b.len()..16 {
                crate::VGA.print_str("   ");
            }
            for d in b {
                let c = *d as char;
                if c.is_ascii() {
                    match *d {
                        32..127 => {
                            crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!("{}", c))
                        }
                        _ => crate::VGA.print_str("?"),
                    }
                } else {
                    crate::VGA.print_str("?");
                }
            }
        }
        crate::VGA.print_str("\r\n");
    }
}

/// prints out a user friendly hex dump of the specified data
pub fn hex_dump_generic_slice<T>(data: &[T], print_address: bool, print_ascii: bool) {
    let len = data.len() * core::mem::size_of::<T>();
    let data =
        unsafe { core::slice::from_raw_parts((data as *const [T] as *const T) as *const u8, len) };
    hex_dump(data, print_address, print_ascii);
}

/// prints out a user friendly hex dump of the specified data
pub fn hex_dump_generic<T>(data: &T, print_address: bool, print_ascii: bool) {
    let len = core::mem::size_of::<T>();
    let data = unsafe { core::slice::from_raw_parts((data as *const T) as *const u8, len) };
    hex_dump(data, print_address, print_ascii);
}

/// prints out a user friendly hex dump of the specified data
pub async fn hex_dump_async(data: &[u8], print_address: bool, print_ascii: bool) {
    let len = data.len();
    let mut addr_len = 1;
    let mut len_calc = len;
    loop {
        if len_calc > 15 {
            addr_len += 1;
            len_calc /= 16;
        } else {
            break;
        }
    }
    crate::VGA
        .print_str_async(&alloc::format!(
            "ADDRESS IS {:p}, size {:x}\r\n",
            data,
            data.len()
        ))
        .await;
    for (i, b) in data.chunks(16).enumerate() {
        if print_address {
            crate::VGA
                .print_str_async(&alloc::format!("{:0>addr_len$x}: ", i * 16))
                .await;
        }
        for d in b {
            crate::VGA
                .print_str_async(&alloc::format!("{:02x} ", d))
                .await;
        }
        if print_ascii {
            for _ in b.len()..16 {
                crate::VGA.print_str_async("   ").await;
            }
            for d in b {
                let c = *d as char;
                if c.is_ascii() {
                    match *d {
                        32..127 => crate::VGA.print_str_async(&alloc::format!("{}", c)).await,
                        _ => crate::VGA.print_str_async("?").await,
                    }
                } else {
                    crate::VGA.print_str_async("?").await;
                }
            }
        }
        crate::VGA.print_str_async("\r\n").await;
    }
}

/// prints out a user friendly hex dump of the specified data
pub async fn hex_dump_generic_async<T>(data: &T, print_address: bool, print_ascii: bool) {
    let len = core::mem::size_of::<T>();
    let data = unsafe { core::slice::from_raw_parts((data as *const T) as *const u8, len) };
    hex_dump_async(data, print_address, print_ascii).await;
}

/// prints out a user friendly hex dump of the specified data
pub async fn hex_dump_generic_slice_async<T>(data: &[T], print_address: bool, print_ascii: bool) {
    let len = data.len() * core::mem::size_of::<T>();
    let data =
        unsafe { core::slice::from_raw_parts((data as *const [T] as *const T) as *const u8, len) };
    hex_dump_async(data, print_address, print_ascii).await;
}

//! Video related kernel modules

pub mod text;
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod vga;

pub mod mipi_dsi;

/// Represents a screen resolution
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

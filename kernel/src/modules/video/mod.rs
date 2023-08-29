//! Video related kernel modules

pub mod text;
pub mod vga;

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

//! Defines the types of pixels that can exist

/// Represents a full color capable pixel, based on the size of P
pub struct FullColor<P> {
    /// The actual full color pixel
    pub pixel: P,
}

impl<P> FullColor<P> {
    /// Make a new pixel
    pub fn new(p: P) -> Self {
        Self { pixel: p }
    }
}

/// Represents a paletted pixel
pub struct Palette<P> {
    pixel: P,
    palette: &'static [u8],
}

impl<P> Palette<P>
where
    P: Into<usize>,
{
    /// Construct a new pixel
    pub fn new(p: P, palette: &'static [u8]) -> Self {
        Self { pixel: p, palette }
    }

    /// Get the pixel value
    pub fn pixel(&self) -> P
    where
        P: Copy,
    {
        self.pixel
    }
}

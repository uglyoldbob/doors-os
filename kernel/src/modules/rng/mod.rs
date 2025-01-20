//! Random number generation code

use crate::LockedArc;
use crate::modules::video::TextDisplayTrait;

/// The standard trait for serial ports
#[enum_dispatch::enum_dispatch]
pub trait RngTrait {
    /// Setup the rng device
    fn setup(&self) -> Result<(), ()>;
    /// Generate some random data
    fn generate(&self, data: &mut [u8]);
    /// Generate some random data into a byte iterator
    fn generate_iter(&self, i: core::slice::IterMut<u8>);
}

/// An enumeration of all the types of serial controllers
#[enum_dispatch::enum_dispatch(RngTrait)]
pub enum Rng {
    /// A typical 32-bit lfsr implementation of random number generation
    Lfsr(LockedArc<RngLfsr>),
}

/// A dummy serial port that does nothing
pub struct RngLfsr {
    next: u32,
}

impl RngLfsr {
    fn advance(&mut self) -> u32 {
        let v = self.next;
        let calc = !((self.next >> 31) ^ (self.next >> 21) ^ (self.next>>1) ^ (self.next & 1));
        self.next = (self.next << 1) | (calc & 1);
        v
    }

    /// Build a new LFSR object
    pub fn new() -> Self {
        Self {
            next: 1,
        }
    }
}

impl RngTrait for LockedArc<RngLfsr> {
    fn setup(&self) -> Result<(),()> {
        let mut s = self.lock();
        s.next = 1;
        Ok(())
    }

    fn generate(&self, data: &mut [u8]) {
        let mut s = self.lock();
        for a in data {
            *a = (s.advance() & 0xFF) as u8;
        }
    }

    fn generate_iter(&self, i: core::slice::IterMut<u8>) {
        let mut s = self.lock();
        for a in i {
            *a = (s.advance() & 0xFF) as u8;
        }
    }
}

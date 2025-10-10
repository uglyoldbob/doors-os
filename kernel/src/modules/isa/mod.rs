//! Code for the ISA bus
//!

pub mod piix3;

/// The trait that defines common functionality for ISA bus adapters
#[enum_dispatch::enum_dispatch]
pub trait IsaBusTrait {
    /// A placeholder function
    fn test(&self) -> bool;
}

/// An ISA bus adapter
#[enum_dispatch::enum_dispatch(IsaBusTrait)]
pub enum IsaBus {
    /// The piix3 isa bus
    Piix3(piix3::IsaPiix3Bridge),
}

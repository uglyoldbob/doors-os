//! Code for the ISA bus
//!
doors_macros::declare_enum!(IsaBus);

pub mod piix3;
doors_macros2::enum_export_builder! {
    doors_macros2::enum_reexport!(PciFunctionDriver, piix3);
    doors_macros2::enum_reexport!(IsaBus, piix3);
}

/// The trait that defines common functionality for ISA bus adapters
#[enum_dispatch::enum_dispatch]
pub trait IsaBusTrait {
    /// A placeholder function
    fn test(&self) -> bool;
}

/// An ISA bus adapter
#[doors_macros::fill_enum_with_variants(IsaBusTrait)]
pub enum IsaBus {}

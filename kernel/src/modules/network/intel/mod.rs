//! Intel drivers for networking hardware

#[doors_macros::reexport_enum_variants()]
mod pro1000;

pub use pro1000::IntelPro1000;
pub use pro1000::IntelPro1000Device;

//! Intel drivers for networking hardware

#[doors_macros::module_builtin_attr(intelpro1000, "true")]
pub mod pro1000;

#[doors_macros::module_builtin_attr(intelpro1000, "true")]
pub use pro1000::IntelPro1000;
#[doors_macros::module_builtin_attr(intelpro1000, "true")]
pub use pro1000::IntelPro1000Device;

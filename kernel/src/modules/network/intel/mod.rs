//! Intel drivers for networking hardware

mod pro1000;

doors_macros2::enum_export_builder! {
    doors_macros2::enum_reexport!(PciFunctionDriver, pro1000);
    doors_macros2::enum_reexport!(NetworkAdapter, pro1000);
}

pub use pro1000::IntelPro1000;
pub use pro1000::IntelPro1000Device;

//! Intel drivers for networking hardware

mod pro1000;

pub use pro1000::IntelPro1000;

/// Represents an intel networking adapter
pub enum IntelNetworkAdapter {
    /// The pro/1000 network adapter
    Pro1000(pro1000::IntelPro1000Device),
}

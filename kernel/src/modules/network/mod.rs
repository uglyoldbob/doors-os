//! Networking code for the kernel

use alloc::{collections::btree_map::BTreeMap, string::String};

use crate::Locked;

pub mod intel;

/// Represents all network adapters for the kernel
const NETWORK_ADAPTERS: Locked<BTreeMap<String, NetworkAdapter>> = Locked::new(BTreeMap::new());

/// Register a network adapter
pub fn register_network_adapter(na: NetworkAdapter) {
    let netad = NETWORK_ADAPTERS;
    let mut nal = netad.lock();
    let name = alloc::format!("net0");
    nal.insert(name, na);
}

/// Represents a network adapter of some variety
pub enum NetworkAdapter {
    /// An intel network adapter
    Intel(intel::IntelNetworkAdapter),
}

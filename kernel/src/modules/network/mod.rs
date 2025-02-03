//! Networking code for the kernel

use crate::modules::video::TextDisplayTrait;

use alloc::{borrow::ToOwned, collections::btree_map::BTreeMap, string::String};

use crate::{Locked, LockedArc};

pub mod intel;

/// Represents all network adapters for the kernel
static NETWORK_ADAPTERS: Locked<BTreeMap<String, LockedArc<NetworkAdapter>>> =
    Locked::new(BTreeMap::new());

/// Register a network adapter
pub fn register_network_adapter(na: NetworkAdapter) {
    let mut nal = NETWORK_ADAPTERS.lock();
    //TODO implement an automatic naming scheme
    let name = alloc::format!("net0");
    doors_macros2::kernel_print!("Registering a network adapter for {}\r\n", name);
    nal.insert(name, LockedArc::new(na));
}

/// Grab a network adapter by name
pub fn get_network_adapter(s: &str) -> Option<LockedArc<NetworkAdapter>> {
    let nal = NETWORK_ADAPTERS.lock();
    for s in nal.keys() {
        doors_macros2::kernel_print!("There is a network adapter {}\r\n", s);
    }
    if nal.contains_key(s) {
        Some(nal.get(s).unwrap().to_owned())
    } else {
        None
    }
}

/// A mac address for a network adapter
pub struct MacAddress {
    address: [u8; 6],
}

/// The trait that defines common functionality for network adapters
#[enum_dispatch::enum_dispatch]
pub trait NetworkAdapterTrait {
    /// Retrieve the mac address for the network adapter
    fn get_mac_address(&mut self) -> MacAddress;
}

/// Represents a network adapter of some variety
#[enum_dispatch::enum_dispatch(NetworkAdapterTrait)]
pub enum NetworkAdapter {
    /// An intel network adapter
    IntelPro1000(intel::IntelPro1000Device),
}

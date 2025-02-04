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

/// An Ipv4 address
pub struct IpV4 {
    /// The 4 parts of the address
    address: [u8; 4],
    /// The subnet mask
    mask: [u8; 4],
}

impl alloc::fmt::Debug for IpV4 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&alloc::format!(
            "{}.{}.{}.{} {}.{}.{}.{}",
            self.address[0],
            self.address[1],
            self.address[2],
            self.address[3],
            self.mask[0],
            self.mask[1],
            self.mask[2],
            self.mask[3]
        ))
    }
}

/// An Ipv6 address
pub struct IpV6 {
    /// The 4 parts of the address
    address: [u16; 8],
    /// The prefix length
    prefix: u8,
}

impl alloc::fmt::Debug for IpV6 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut zeros: [bool; 6] = [false; 6];
        for i in 0..6 {
            zeros[i] = self.address[i] == 0;
        }
        let mut num_consecutive_zeros = [0; 6];
        {
            for i in 0..6 {
                if zeros[i] {
                    let mut j = i;
                    loop {
                        if i >= 6 {
                            break;
                        }
                        if !zeros[j] {
                            break;
                        }
                        j += 1;
                    }
                }
            }
        }

        Ok(())
    }
}

#[doors_macros::doors_test]
fn ipv6_network_test() -> Result<(), ()> {
    Err(())
}

/// A network adapter ip address
pub enum IpAddress {
    /// An ipv4 address
    IpV4(IpV4),
    /// An ipv6 address
    IpV6(IpV6),
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

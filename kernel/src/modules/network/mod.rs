//! Networking code for the kernel

use alloc::{borrow::ToOwned, collections::btree_map::BTreeMap, string::String};

use crate::{AsyncLocked, AsyncLockedArc, LockedArc};

doors_macros::declare_enum!(NetworkAdapter);

pub mod intel;

doors_macros2::enum_reexport!(intel);

lazy_static::lazy_static! {
    /// Represents all network adapters for the kernel
    static ref NETWORK_ADAPTERS: AsyncLocked<BTreeMap<String, AsyncLockedArc<NetworkAdapter>>> =
        AsyncLocked::new(BTreeMap::new());
}

/// Register a network adapter
pub async fn register_network_adapter(na: NetworkAdapter) {
    let mut nal = NETWORK_ADAPTERS.lock().await;
    //TODO implement an automatic naming scheme
    use alloc::string::ToString;
    let name = "net0".to_string();
    crate::VGA
        .print_str_async(&alloc::format!(
            "Registering a network adapter for {}\r\n",
            name
        ))
        .await;
    nal.insert(name, AsyncLockedArc::new(na));
}

/// Grab a network adapter by name
pub async fn get_network_adapter(s: &str) -> Option<AsyncLockedArc<NetworkAdapter>> {
    let nal = NETWORK_ADAPTERS.lock().await;
    if nal.contains_key(s) {
        Some(nal.get(s).unwrap().to_owned())
    } else {
        None
    }
}

/// A mac address for a network adapter
#[derive(Clone, Copy, Debug)]
pub struct MacAddress {
    /// The bytes of the mac address
    address: [u8; 6],
}

impl Default for MacAddress {
    fn default() -> Self {
        Self { address: [0; 6] }
    }
}

impl From<u64> for MacAddress {
    fn from(value: u64) -> Self {
        let a = value.to_le_bytes();
        Self {
            address: [a[0], a[1], a[2], a[3], a[4], a[5]],
        }
    }
}

impl From<MacAddress> for u64 {
    fn from(value: MacAddress) -> u64 {
        let a: [u8; 8] = [
            value.address[0],
            value.address[1],
            value.address[2],
            value.address[3],
            value.address[4],
            value.address[5],
            0,
            0,
        ];
        u64::from_le_bytes(a)
    }
}

/// Test the mac address conversion to and from u64
#[doors_macros::doors_test]
fn mac_address_conversion_test() -> Result<(), ()> {
    let mac = MacAddress {
        address: [1, 2, 3, 4, 5, 6],
    };
    let b: u64 = mac.into();
    assert_eq!(b, 0x060504030201);
    let mac2: MacAddress = b.into();
    assert_eq!(mac.address, mac2.address);
    Ok(())
}

/// The trait that defines common functionality for network adapters
#[enum_dispatch::enum_dispatch]
pub trait NetworkAdapterTrait {
    /// Retrieve the mac address for the network adapter
    fn get_mac_address(&mut self) -> MacAddress;
    /// Send a packet over the network interface
    async fn send_packet(&mut self, packet: &[u8]) -> Result<(), ()>;
}

/// A network adapter
#[doors_macros::fill_enum_with_variants(NetworkAdapterTrait)]
pub enum NetworkAdapter {}

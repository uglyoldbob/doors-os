//! Networking code for the kernel

use alloc::{borrow::ToOwned, collections::btree_map::BTreeMap, string::String};

use crate::{AsyncLocked, AsyncLockedArc, IrqGuardedSimple, LockedArc};

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
    async fn get_mac_address(&mut self) -> MacAddress;
    /// Send a packet over the network interface
    async fn send_packet(&mut self, packet: &[u8]) -> Result<(), ()>;
}

/// A network adapter
#[doors_macros::fill_enum_with_variants(NetworkAdapterTrait)]
pub enum NetworkAdapter {}

const MAX_RX_PACKET_SIZE: usize = 8192;

/// A raw ethernet packet received from a network card
pub struct EthernetPacket {
    /// The contents of the packet
    data: [u8; MAX_RX_PACKET_SIZE],
    /// The actual length of the packet
    length: usize,
}

impl core::fmt::Debug for EthernetPacket {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for i in 0..self.length {
            let d = self.data[i];
            f.write_str(&alloc::format!("{:x} ", d))?;
        }
        Ok(())
    }
}

impl EthernetPacket {
    fn new() -> Self {
        Self {
            data: [0; MAX_RX_PACKET_SIZE],
            length: 0,
        }
    }

    /// Copy the data into the packet for processing
    fn copy(&mut self, r: &[u8]) {
        self.data[0..r.len()].copy_from_slice(r);
        self.length = r.len();
    }
}

lazy_static::lazy_static! {
    /// The list of received packets
    pub static ref ETHERNET_PACKETS_RECEIVED: conquer_once::spin::OnceCell<crossbeam::queue::ArrayQueue<EthernetPacket>> =
        conquer_once::spin::OnceCell::uninit();
}

/// Initialize data required for network operations
pub fn network_init() {
    ETHERNET_PACKETS_RECEIVED.init_once(|| {
        crossbeam::queue::ArrayQueue::new(32)
    });
}

/// Temporary function to process received ethernet packets
pub async fn process_packets_received() {
    loop {
        if let Some(q) = ETHERNET_PACKETS_RECEIVED.get() {
            if let Some(p) = q.pop() {
                crate::VGA.print_str(&alloc::format!("Received packet: {:?}\r\n", p));
            }
            else {
                crate::executor::Task::yield_now().await;
            }
        }
    }
}

/// Called from an interrupt context to process a received ethernet packet
fn interrupt_process_received_packet(packet: EthernetPacket) {
    if let Some(q) = ETHERNET_PACKETS_RECEIVED.get() {
        let _ = q.push(packet);
    }
}

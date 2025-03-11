//! Networking code for the kernel

use alloc::{
    borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, string::String, vec::Vec,
};

use crate::{Arc, AsyncLocked, AsyncLockedArc, IrqGuarded};

doors_macros::declare_enum!(NetworkAdapter);

pub mod intel;

doors_macros2::enum_export_builder! {
    doors_macros2::enum_reexport!(PciFunctionDriver, intel);
    doors_macros2::enum_reexport!(NetworkAdapter, intel);
}

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
#[derive(Clone, Copy, Debug, Default)]
pub struct MacAddress {
    /// The bytes of the mac address
    address: [u8; 6],
}

impl From<&[u8]> for MacAddress {
    fn from(a: &[u8]) -> Self {
        Self {
            address: [a[0], a[1], a[2], a[3], a[4], a[5]],
        }
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

/// The maximim amount of data to receive in a single packet
const MAX_RX_PACKET_SIZE: usize = 8192;

/// An ethernet packet header
#[derive(Debug)]
#[allow(unused)]
pub struct EthernetFrameHeader {
    /// The destination for the packet
    destination: MacAddress,
    /// The source of the packet
    source: MacAddress,
    /// The optional 802.1q vlan data
    vlan: Option<u32>,
    /// The type of the packet
    ethertype: u16,
}

/// Represents a received ethernet frame
#[derive(Debug)]
#[allow(unused)]
pub struct EthernetFrame<'a> {
    /// The header
    header: EthernetFrameHeader,
    /// The actual packet data
    data: &'a [u8],
    /// The crc of the packet
    crc: u32,
}

/// A raw ethernet packet received from a network card
#[derive(Clone)]
pub struct RawEthernetPacket {
    /// The contents of the packet
    data: [u8; MAX_RX_PACKET_SIZE],
    /// The actual length of the packet
    length: usize,
}

impl<'a> From<&'a RawEthernetPacket> for EthernetFrame<'a> {
    fn from(value: &'a RawEthernetPacket) -> Self {
        doors_macros::todo_item!("Process 802.1q information present in frame");
        let d = &value.data[0..6];
        let s = &value.data[6..12];
        let header = EthernetFrameHeader {
            destination: d.into(),
            source: s.into(),
            vlan: None,
            ethertype: u16::from_le_bytes([value.data[12], value.data[13]]),
        };
        let l = value.length - 18;
        let dat = &value.data[14..(14 + l)];
        let crc = u32::from_le_bytes([
            value.data[14 + l],
            value.data[15 + l],
            value.data[16 + l],
            value.data[17 + l],
        ]);
        Self {
            header,
            data: dat,
            crc,
        }
    }
}

impl core::fmt::Debug for RawEthernetPacket {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for i in 0..self.length {
            let d = self.data[i];
            f.write_str(&alloc::format!("{:x} ", d))?;
        }
        Ok(())
    }
}

impl RawEthernetPacket {
    /// Construct a new boxed empty packet, without allocating any memory on the stack
    fn new_box() -> alloc::boxed::Box<Self> {
        unsafe {
            let layout = alloc::alloc::Layout::new::<Self>();
            let ptr = alloc::alloc::alloc(layout) as *mut Self;
            (*ptr).length = 0;
            alloc::boxed::Box::from_raw(ptr)
        }
    }

    /// Copy the data into the packet for processing
    fn copy(&mut self, r: &[u8]) {
        self.data[0..r.len()].copy_from_slice(r);
        self.length = r.len();
    }
}

/// A structure to received packets from a network interface
pub struct NetworkReceiver {
    /// The list of packets received from the network card
    packets: alloc::collections::vec_deque::VecDeque<Box<RawEthernetPacket>>,
}

impl NetworkReceiver {
    /// Construct a new self
    fn new() -> Self {
        Self {
            packets: alloc::collections::vec_deque::VecDeque::new(),
        }
    }
}

lazy_static::lazy_static! {
    /// The list of received packets
    pub static ref ETHERNET_PACKETS_RECEIVED: AsyncLocked<Vec<Arc<IrqGuarded<NetworkReceiver>>>> = AsyncLocked::new(Vec::new());
}

/// Initialize data required for network operations, returning the index of the registered network packet receiver
fn network_init(i: Arc<IrqGuarded<NetworkReceiver>>) {
    let mut e = ETHERNET_PACKETS_RECEIVED.sync_lock();
    e.push(i);
}

/// Temporary function to process received ethernet packets
pub async fn process_packets_received() {
    loop {
        let mut e = ETHERNET_PACKETS_RECEIVED.lock().await;
        let mut received_packet = false;
        for ethernet in e.iter_mut() {
            let mut ethernet = ethernet.access().await;
            while let Some(packet) = ethernet.packets.pop_front() {
                received_packet = true;
                crate::VGA
                    .print_str_async(&alloc::format!("Received packet: {:x?}\r\n", packet))
                    .await;
            }
        }
        drop(e);
        if !received_packet {
            for _ in 0..1000000 {
                crate::executor::AsyncTask::yield_now().await;
            }
        }
    }
}

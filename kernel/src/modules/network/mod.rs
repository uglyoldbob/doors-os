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
#[derive(Clone, Debug)]
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

/// A udp packet
#[derive(Debug)]
pub struct UdpPacket<'a> {
    /// the source port
    source: u16,
    /// the destination port
    destination: u16,
    /// The length of the packet, including the 8 byte header
    length: u16,
    /// The packet checksum
    checksum: u16,
    /// The actual packet data
    data: &'a [u8],
}

impl<'a> From<&'a [u8]> for UdpPacket<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self {
            source: u16::from_be_bytes([value[0], value[1]]),
            destination: u16::from_be_bytes([value[2], value[3]]),
            length: u16::from_be_bytes([value[4], value[5]]),
            checksum: u16::from_be_bytes([value[6], value[7]]),
            data: &value[8..],
        }
    }
}

/// Represents the types of data that a tcp packet can contain
#[derive(Debug)]
pub enum IpPacketData<'a> {
    /// Internet control message protocol
    Icmp(&'a [u8]),
    /// Internet group management protocol
    Igmp(&'a [u8]),
    /// Transmission control protocol
    Tcp(&'a [u8]),
    /// User datagram protocol
    Udp(UdpPacket<'a>),
    /// Ipv6 encapsulated packet
    Ipv6Encapsulated(&'a [u8]),
    /// open shortest path first
    Ospf(&'a [u8]),
    /// stream control transmission protocol
    Sctp(&'a [u8]),
    /// Unknown protocol
    Unknown(&'a [u8]),
}

/// Defines the layout of an ipv4 packet header
#[derive(Debug)]
pub struct Ipv4PacketHeader {
    /// Packet version, actually 4 bits
    version: u8,
    /// The header size in number of u32
    header_size: u8,
    /// differentiated services code point (6 bits)
    dscp: u8,
    /// explicit congestion notification (2 bits)
    ecn: u8,
    /// Total length of the packet, including the header. Valid values are 20..=65535
    total_length: u16,
    /// identification for id of a single ip datagram
    id: u16,
    /// flags regarding fragmentation
    flags: u8,
    /// fragment offset
    fragment_offset: u16,
    /// time to live
    ttl: u8,
    /// transport layer protocol
    protocol: u8,
    /// header chuecksum
    checksum: u16,
    /// The source ip address
    source: u32,
    /// The destination ip address
    destination: u32,
    /// The packet options
    options: [u32; 10],
}

impl From<&[u8]> for Ipv4PacketHeader {
    fn from(value: &[u8]) -> Self {
        let header_size = value[0] & 0xf;
        let mut options: [u32; 10] = [0; 10];
        if header_size > 5 {
            for (i, c) in value[20..].chunks_exact(4).enumerate() {
                let u: u32 = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
                options[i] = u;
            }
        }
        Self {
            version: value[0] >> 4,
            header_size,
            dscp: value[1] >> 2,
            ecn: value[1] & 3,
            total_length: u16::from_be_bytes([value[2], value[3]]),
            id: u16::from_be_bytes([value[4], value[5]]),
            flags: value[6] >> 5,
            fragment_offset: u16::from_be_bytes([value[6], value[7]]) & 0x1FFF,
            ttl: value[8],
            protocol: value[9],
            checksum: u16::from_be_bytes([value[10], value[11]]),
            source: u32::from_be_bytes([value[12], value[13], value[14], value[15]]),
            destination: u32::from_be_bytes([value[16], value[17], value[18], value[19]]),
            options,
        }
    }
}

/// An ip version 4 packet
#[derive(Debug)]
pub struct Ipv4Packet<'a> {
    /// The packet header
    header: Ipv4PacketHeader,
    /// The packet data
    data: IpPacketData<'a>,
}

impl<'a> From<&'a [u8]> for Ipv4Packet<'a> {
    fn from(value: &'a [u8]) -> Self {
        let header = Ipv4PacketHeader::from(value);
        let data_start = (4 * header.header_size as u16) as usize;
        let data_length = header.total_length as usize - data_start;
        let raw_data = &value[data_start..data_start + data_length];
        let data = match header.protocol {
            1 => IpPacketData::Icmp(raw_data),
            2 => IpPacketData::Igmp(raw_data),
            6 => IpPacketData::Tcp(raw_data),
            17 => IpPacketData::Udp(raw_data.into()),
            41 => IpPacketData::Ipv6Encapsulated(raw_data),
            89 => IpPacketData::Ospf(raw_data),
            132 => IpPacketData::Sctp(raw_data),
            _ => IpPacketData::Unknown(raw_data),
        };
        Self { header, data }
    }
}

/// THe format of an address resolution packet
#[derive(Debug)]
pub struct AddressResolutionProtocolPacket<'a> {
    /// The hwrdware type
    htype: u16,
    /// protocol type
    ptype: u16,
    /// The length of a hardware address
    address_length: u8,
    /// The length of the protocol address
    protocol_length: u8,
    /// The operation for the sender of the packet
    operation: u16,
    /// The sender hardware address
    sender_hardware_address: &'a [u8],
    /// The sender protocol address
    sender_protocol_address: &'a [u8],
    /// The target hardware address
    target_hardware_address: &'a [u8],
    /// The target protocol address
    target_protocol_address: &'a [u8],
}

impl<'a> From<&'a [u8]> for AddressResolutionProtocolPacket<'a> {
    fn from(value: &'a [u8]) -> Self {
        let hlength = value[4] as usize;
        let plength = value[5] as usize;
        let mut offset = 8;
        let sha = &value[offset..offset + hlength];
        offset += hlength;
        let spa = &value[offset..offset + plength];
        offset += plength;
        let tha = &value[offset..offset + hlength];
        offset += hlength;
        let tpa = &value[offset..offset + plength];
        Self {
            htype: u16::from_be_bytes([value[0], value[1]]),
            ptype: u16::from_be_bytes([value[2], value[3]]),
            address_length: hlength as u8,
            protocol_length: plength as u8,
            operation: u16::from_be_bytes([value[6], value[7]]),
            sender_hardware_address: sha,
            sender_protocol_address: spa,
            target_hardware_address: tha,
            target_protocol_address: tpa,
        }
    }
}

/// The vaious types of packets that can exist
#[derive(Debug)]
pub enum Packet<'a> {
    /// An ip version 4 packet
    Ipv4(Ipv4Packet<'a>),
    /// Address resolution protocol
    Arp(AddressResolutionProtocolPacket<'a>),
    /// The packet type is unknown, but here is the data anyways, have fun!
    Unknown(&'a [u8]),
}

/// Represents a decoded ethernet frame
#[derive(Debug)]
pub struct DecodedEthernetFrame<'a> {
    /// The header
    header: EthernetFrameHeader,
    /// The packet contents
    contents: Packet<'a>,
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

impl<'a> From<&'a EthernetFrame<'a>> for DecodedEthernetFrame<'a> {
    fn from(value: &'a EthernetFrame<'a>) -> Self {
        match value.header.ethertype {
            8 => Self {
                header: value.header.clone(),
                contents: Packet::Ipv4(Ipv4Packet::from(value.data)),
            },
            1544 => Self {
                header: value.header.clone(),
                contents: Packet::Arp(value.data.into()),
            },
            _ => Self {
                header: value.header.clone(),
                contents: Packet::Unknown(value.data),
            },
        }
    }
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
                let ep: EthernetFrame = packet.as_ref().into();
                let df: DecodedEthernetFrame = (&ep).into();
                match df.contents {
                    Packet::Ipv4(ipv4_packet) => {}
                    Packet::Arp(address_resolution_protocol_packet) => {
                        crate::VGA
                            .print_str_async(&alloc::format!(
                                "Received arp packet: {:02x?}\r\n",
                                address_resolution_protocol_packet
                            ))
                            .await;
                    }
                    Packet::Unknown(items) => {}
                }
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

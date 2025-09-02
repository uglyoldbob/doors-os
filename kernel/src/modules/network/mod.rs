//! Networking code for the kernel

use alloc::{
    borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, string::String,
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
    let na = AsyncLockedArc::new(na);
    let rxtx = na.make_transceiver().await;
    network_init(rxtx).await;
    nal.insert(name, na);
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
    /// Get the receiver clone
    fn get_receiver(&self) -> Arc<IrqGuarded<NetworkReceiver>>;
}

impl AsyncLockedArc<NetworkAdapter> {
    /// Build a network transceiver
    pub async fn make_transceiver(&self) -> NetworkTransceiver {
        NetworkTransceiver {
            receiver: self.lock().await.get_receiver(),
            sender: self.clone(),
        }
    }
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

impl<'a> TryFrom<&'a [u8]> for UdpPacket<'a> {
    type Error = ();
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            source: u16::from_be_bytes([value[0], value[1]]),
            destination: u16::from_be_bytes([value[2], value[3]]),
            length: u16::from_be_bytes([value[4], value[5]]),
            checksum: u16::from_be_bytes([value[6], value[7]]),
            data: value.get(8..).ok_or(())?,
        })
    }
}

/// A tcp packet
#[derive(Debug)]
pub struct TcpPacket<'a> {
    /// The source port
    source: u16,
    /// The destination port
    destination: u16,
    /// Sequence number
    sequence: u32,
    /// acknowledgement number
    ack: u32,
    /// data offset
    data_offset: u8,
    /// various flags
    flags: u8,
    /// Receive window size
    window: u16,
    /// checksum
    checksum: u16,
    /// urgent pointer
    urgent: u16,
    /// The packet options
    options: [u32; 10],
    /// Packet data
    data: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for TcpPacket<'a> {
    type Error = ();
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        let data_offset = *value.get(12).ok_or(())? >> 4;
        let mut options: [u32; 10] = [0; 10];
        if data_offset > 5 {
            for (i, c) in value
                .get(20..)
                .ok_or(())?
                .chunks_exact(4)
                .enumerate()
                .take(data_offset as usize - 5)
            {
                let u: u32 = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
                options[i] = u;
            }
        }
        Ok(Self {
            source: u16::from_be_bytes(value.get(0..=1).ok_or(())?.try_into().map_err(|_| ())?),
            destination: u16::from_be_bytes(
                value.get(2..=3).ok_or(())?.try_into().map_err(|_| ())?,
            ),
            sequence: u32::from_be_bytes(value.get(4..=7).ok_or(())?.try_into().map_err(|_| ())?),
            ack: u32::from_be_bytes(value.get(8..=11).ok_or(())?.try_into().map_err(|_| ())?),
            data_offset,
            flags: *value.get(13).ok_or(())?,
            window: u16::from_be_bytes(value.get(14..=15).ok_or(())?.try_into().map_err(|_| ())?),
            checksum: u16::from_be_bytes(value.get(16..=17).ok_or(())?.try_into().map_err(|_| ())?),
            urgent: u16::from_be_bytes(value.get(18..=19).ok_or(())?.try_into().map_err(|_| ())?),
            options,
            data: value.get(data_offset as usize * 4..).ok_or(())?,
        })
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
    Tcp(TcpPacket<'a>),
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

impl TryFrom<&[u8]> for Ipv4PacketHeader {
    type Error = ();
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let header_size = value.get(0).ok_or(())? & 0xf;
        let mut options: [u32; 10] = [0; 10];
        if header_size > 5 {
            for (i, c) in value.get(20..).ok_or(())?.chunks_exact(4).enumerate() {
                let u: u32 = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
                options[i] = u;
            }
        }
        Ok(Self {
            version: value.get(0).ok_or(())? >> 4,
            header_size,
            dscp: value.get(1).ok_or(())? >> 2,
            ecn: value.get(1).ok_or(())? & 3,
            total_length: u16::from_be_bytes([*value.get(2).ok_or(())?, *value.get(3).ok_or(())?]),
            id: u16::from_be_bytes([*value.get(4).ok_or(())?, *value.get(5).ok_or(())?]),
            flags: value.get(6).ok_or(())? >> 5,
            fragment_offset: u16::from_be_bytes([
                *value.get(6).ok_or(())?,
                *value.get(7).ok_or(())?,
            ]) & 0x1FFF,
            ttl: *value.get(8).ok_or(())?,
            protocol: *value.get(9).ok_or(())?,
            checksum: u16::from_be_bytes([*value.get(10).ok_or(())?, *value.get(11).ok_or(())?]),
            source: u32::from_be_bytes([
                *value.get(12).ok_or(())?,
                *value.get(13).ok_or(())?,
                *value.get(14).ok_or(())?,
                *value.get(15).ok_or(())?,
            ]),
            destination: u32::from_be_bytes([
                *value.get(16).ok_or(())?,
                *value.get(17).ok_or(())?,
                *value.get(18).ok_or(())?,
                *value.get(19).ok_or(())?,
            ]),
            options,
        })
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

impl<'a> TryFrom<&'a [u8]> for Ipv4Packet<'a> {
    type Error = ();
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        let header = Ipv4PacketHeader::try_from(value)?;
        let data_start = (4 * header.header_size as u16) as usize;
        let data_length = header.total_length as usize - data_start;
        let raw_data = value.get(data_start..data_start + data_length).ok_or(())?;
        let data = match header.protocol {
            1 => IpPacketData::Icmp(raw_data),
            2 => IpPacketData::Igmp(raw_data),
            6 => {
                let p = raw_data.try_into();
                if p.is_err() {
                    crate::VGA.print_str("Failed to decode tcp packet\r\n");
                }
                IpPacketData::Tcp(p?)
            }
            17 => IpPacketData::Udp(raw_data.try_into()?),
            41 => IpPacketData::Ipv6Encapsulated(raw_data),
            89 => IpPacketData::Ospf(raw_data),
            132 => IpPacketData::Sctp(raw_data),
            _ => IpPacketData::Unknown(raw_data),
        };
        Ok(Self { header, data })
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

impl<'a> AddressResolutionProtocolPacket<'a> {
    /// construct a packet in the specified buffer
    pub fn build_packet(&self, packet: &mut [u8]) -> Result<u16, ()> {
        let mut offset = 0;
        (&mut packet[offset..offset + 2]).copy_from_slice(&self.htype.to_be_bytes());
        offset += 2;
        (&mut packet[offset..offset + 2]).copy_from_slice(&self.ptype.to_be_bytes());
        offset += 2;
        (&mut packet[offset..offset + 1]).copy_from_slice(&self.address_length.to_be_bytes());
        offset += 1;
        (&mut packet[offset..offset + 1]).copy_from_slice(&self.protocol_length.to_be_bytes());
        offset += 1;
        (&mut packet[offset..offset + 2]).copy_from_slice(&self.operation.to_be_bytes());
        offset += 2;
        (&mut packet[offset..offset + self.address_length as usize])
            .copy_from_slice(self.sender_hardware_address);
        offset += self.address_length as usize;
        (&mut packet[offset..offset + self.protocol_length as usize])
            .copy_from_slice(self.sender_protocol_address);
        offset += self.protocol_length as usize;
        (&mut packet[offset..offset + self.address_length as usize])
            .copy_from_slice(self.target_hardware_address);
        offset += self.address_length as usize;
        (&mut packet[offset..offset + self.protocol_length as usize])
            .copy_from_slice(self.target_protocol_address);
        offset += self.protocol_length as usize;
        Ok(offset as u16)
    }
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

impl<'a> Packet<'a> {
    /// construct a packet in the specified buffer
    pub fn build_packet(&self, packet: &mut [u8]) -> Result<u16, ()> {
        match self {
            Packet::Ipv4(ipv4_packet) => todo!(),
            Packet::Arp(address_resolution_protocol_packet) => {
                address_resolution_protocol_packet.build_packet(packet)
            }
            Packet::Unknown(items) => {
                packet.copy_from_slice(*items);
                Ok(items.len() as u16)
            }
        }
    }
}

/// Represents a decoded ethernet frame
#[derive(Debug)]
pub struct DecodedEthernetFrame<'a> {
    /// The header
    header: EthernetFrameHeader,
    /// The packet contents
    contents: Packet<'a>,
}

impl<'a> DecodedEthernetFrame<'a> {
    /// construct a packet in the specified buffer
    pub fn build_packet(&self, packet: &mut [u8]) -> Result<u16, ()> {
        (&mut packet[0..6]).copy_from_slice(&self.header.destination.address);
        (&mut packet[6..12]).copy_from_slice(&self.header.source.address);
        let mut offset = 12;
        if let Some(q) = self.header.vlan {
            (&mut packet[offset..offset + 4]).copy_from_slice(&q.to_be_bytes());
            offset += 4;
        }
        (&mut packet[offset..offset + 2]).copy_from_slice(&self.header.ethertype.to_be_bytes());
        offset += 2;
        offset += self.contents.build_packet(&mut packet[offset..])? as usize;
        Ok(offset as u16)
    }
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

impl<'a> TryFrom<&'a EthernetFrame<'a>> for DecodedEthernetFrame<'a> {
    type Error = ();
    fn try_from(value: &'a EthernetFrame<'a>) -> Result<Self, Self::Error> {
        Ok(match value.header.ethertype {
            0x800 => Self {
                header: value.header.clone(),
                contents: Packet::Ipv4(Ipv4Packet::try_from(value.data)?),
            },
            2054 => Self {
                header: value.header.clone(),
                contents: Packet::Arp(value.data.into()),
            },
            _ => Self {
                header: value.header.clone(),
                contents: Packet::Unknown(value.data),
            },
        })
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

impl<'a> TryFrom<&'a RawEthernetPacket> for EthernetFrame<'a> {
    type Error = ();
    fn try_from(value: &'a RawEthernetPacket) -> Result<Self, Self::Error> {
        doors_macros::todo_item!("Process 802.1q information present in frame");
        let d = value.data.get(0..6).ok_or(())?;
        let s = value.data.get(6..12).ok_or(())?;
        let header = EthernetFrameHeader {
            destination: d.into(),
            source: s.into(),
            vlan: None,
            ethertype: u16::from_be_bytes([
                *value.data.get(12).ok_or(())?,
                *value.data.get(13).ok_or(())?,
            ]),
        };
        let l = value.length - 18;
        let dat = value.data.get(14..(14 + l)).ok_or(())?;
        let crc = u32::from_be_bytes([
            *value.data.get(14 + l).ok_or(())?,
            *value.data.get(15 + l).ok_or(())?,
            *value.data.get(16 + l).ok_or(())?,
            *value.data.get(17 + l).ok_or(())?,
        ]);
        Ok(Self {
            header,
            data: dat,
            crc,
        })
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

/// How packets are sent and received on a single network adapter
pub struct NetworkTransceiver {
    /// How packets are received
    pub receiver: Arc<IrqGuarded<NetworkReceiver>>,
    /// How packets are sent
    pub sender: AsyncLockedArc<NetworkAdapter>,
}

impl NetworkTransceiver {
    /// Process network packets received
    pub async fn run(&self) {
        loop {
            let mut rx = self.receiver.access().await;
            while let Some(packet) = rx.packets.pop_front() {
                if let Ok(ep) = packet.as_ref().try_into() {
                    let ep: EthernetFrame = ep;
                    crate::VGA
                        .print_str_async(&alloc::format!("Received packet: {:02x?}\r\n", ep))
                        .await;
                    if let Ok(df) = (&ep).try_into() {
                        let df: DecodedEthernetFrame = df;
                        crate::VGA
                            .print_str_async(&alloc::format!(
                                "Received packet for: {:02x?}\r\n",
                                df.header.destination
                            ))
                            .await;
                        match df.contents {
                            Packet::Ipv4(ipv4_packet) => {
                                crate::VGA
                                    .print_str_async(&alloc::format!(
                                        "Received ip packet: {:02x?}\r\n",
                                        ipv4_packet
                                    ))
                                    .await;
                            }
                            Packet::Arp(address_resolution_protocol_packet) => {
                                crate::VGA
                                    .print_str_async(&alloc::format!(
                                        "Received arp packet: {:02x?}\r\n",
                                        address_resolution_protocol_packet
                                    ))
                                    .await;
                                if address_resolution_protocol_packet.operation == 1 {
                                    let mymac =
                                        self.sender.lock().await.get_mac_address().await.address;
                                    doors_macros::todo_item!("Populate the actual ip address");
                                    let myip = [11, 11, 11, 12];
                                    if address_resolution_protocol_packet.target_protocol_address
                                        == myip
                                    {
                                        let mut packet = [0; 128];
                                        let p = DecodedEthernetFrame {
                                            header: EthernetFrameHeader {
                                                destination: df.header.source,
                                                source: MacAddress::default(),
                                                vlan: None,
                                                ethertype: 2054,
                                            },
                                            contents: Packet::Arp(
                                                AddressResolutionProtocolPacket {
                                                    htype: address_resolution_protocol_packet.htype,
                                                    ptype: address_resolution_protocol_packet.ptype,
                                                    address_length:
                                                        address_resolution_protocol_packet
                                                            .address_length,
                                                    protocol_length:
                                                        address_resolution_protocol_packet
                                                            .protocol_length,
                                                    operation: 2,
                                                    sender_hardware_address: &mymac,
                                                    sender_protocol_address: &myip,
                                                    target_hardware_address:
                                                        address_resolution_protocol_packet
                                                            .sender_hardware_address,
                                                    target_protocol_address:
                                                        address_resolution_protocol_packet
                                                            .sender_protocol_address,
                                                },
                                            ),
                                        };
                                        if let Ok(length) = p.build_packet(&mut packet) {
                                            let _ = self
                                                .sender
                                                .lock()
                                                .await
                                                .send_packet(&packet[0..length as usize])
                                                .await;
                                        }
                                    }
                                }
                            }
                            Packet::Unknown(stuff) => {
                                crate::VGA
                                    .print_str_async(&alloc::format!(
                                        "Received unknown packet: {:02x?}\r\n",
                                        df
                                    ))
                                    .await;
                            }
                        }
                    }
                    crate::VGA.print_str("Done processing packet\r\n");
                }
            }
            crate::executor::AsyncTask::yield_now().await;
        }
    }
}

/// Initialize data required for network operations
async fn network_init(i: NetworkTransceiver) {
    let _ = crate::executor::spawn(crate::AsyncTask::new(async move {
        i.run().await;
    }));
}

//! Networking code for the kernel

use core::{net::IpAddr, str::FromStr};

use alloc::{borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, string::String};
use futures::StreamExt;

use crate::{
    new_stream, AsyncLocked, AsyncLockedArc, IrqStreamReader, IrqStreamWriter, Locked,
    StreamReader, StreamWriter,
};

pub mod intel;
mod loopback;

lazy_static::lazy_static! {
    /// Represents all network adapters for the kernel
    static ref NETWORK_ADAPTERS: AsyncLocked<BTreeMap<String, IrqStreamWriter<RawEthernetPacket>>> =
        AsyncLocked::new(BTreeMap::new());
    /// The lookup table to convert ip addresses to mac addresses
    static ref IP_TO_MAC_TABLE: AsyncLockedArc<BTreeMap<core::net::IpAddr, Option<MacAddress>>> = AsyncLockedArc::new(BTreeMap::new());
    /// The list of udp ports expecting data
    static ref UDP_PORTS_INCOMING: AsyncLockedArc<BTreeMap<u16, StreamWriter<UdpPacket>>> = AsyncLockedArc::new(BTreeMap::new());
}

/// Register a network adapter
#[cfg_attr(feature = "backtrace", doors_macros::framed)]
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
    crate::VGA
        .print_str_async("Spawning network code\r\n")
        .await;
    let nas = na.get_sender();
    crate::print_locations().await;
    crate::executor::spawn(async {
        crate::VGA
            .print_str_async("Running network1 code\r\n")
            .await;
    })
    .unwrap();
    let r = crate::executor::spawn(async move {
        crate::VGA.print_str_async("Running network code\r\n").await;
        na.run().await;
    });
    crate::print_locations().await;
    crate::VGA
        .print_str_async(&alloc::format!("RESULT Spawning network code: {:?}\r\n", r))
        .await;
    nal.insert(name, nas);
}

/// Grab a network adapter by name
#[cfg_attr(feature = "backtrace", doors_macros::framed)]
pub async fn get_network_adapter(s: &str) -> Option<IrqStreamWriter<RawEthernetPacket>> {
    let nal = NETWORK_ADAPTERS.lock().await;
    if nal.contains_key(s) {
        Some(nal.get(s).unwrap().to_owned())
    } else {
        None
    }
}

/// Get an object for sending and receiving udp traffic
pub fn get_udp(ip: core::net::IpAddr, dest: u16) -> Option<UdpLayer> {
    let nal = NETWORK_ADAPTERS.sync_lock();
    if let Some(a) = nal.first_key_value() {
        let el = EthernetLayer::new(
            MacAddress::broadcast(),
            MacAddress::broadcast(),
            a.1.clone(),
        );
        let ip = Ip4Layer::new(el, ip, IpAddr::from_str("11.11.11.12").unwrap());
        Some(UdpLayer::new(ip, 12345, dest))
    } else {
        None
    }
}

/// A mac address for an ethernet network adapter
#[derive(Clone, Copy, Debug, Default)]
pub struct MacAddress {
    /// The bytes of the mac address
    address: [u8; 6],
}

impl MacAddress {
    /// Get the broadcast mac address
    pub fn broadcast() -> Self {
        Self { address: [0xff; 6] }
    }
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
    /// Get the receiver
    fn get_receiver(&self) -> Option<IrqStreamReader<RawEthernetPacket>>;
    /// Get a new packet sender queue clone
    fn get_sender(&self) -> IrqStreamWriter<RawEthernetPacket>;
    /// Sends pending packets
    async fn send_pending_packets(self);
}

/// A network adapter
#[doors_macros::enum_module_filter]
#[enum_dispatch::enum_dispatch(NetworkAdapterTrait)]
pub enum NetworkAdapter {
    /// The intel pro 1000 device
    #[doors_module = "intelpro1000"]
    IntelPro1000(intel::pro1000::IntelPro1000Device),
    /// a loopback device
    Loopback(loopback::NetworkLoopback),
}

/// Process network packets received
pub async fn receive_packets(
    mymac: MacAddress,
    mut r: IrqStreamReader<RawEthernetPacket>,
    s: IrqStreamWriter<RawEthernetPacket>,
) {
    loop {
        while let Some(packet) = r.next().await {
            if let Ok(ep) = (&packet).try_into() {
                let ep: RawEthernetFrame = ep;
                crate::VGA
                    .print_str_async(&alloc::format!("Received packet: {:02x?}\r\n", ep))
                    .await;
                if let Ok(df) = (&ep).try_into() {
                    let df: EthernetFrame = df;
                    crate::VGA
                        .print_str_async(&alloc::format!(
                            "Received packet for: {:02x?}\r\n",
                            df.header.destination
                        ))
                        .await;
                    match df.contents {
                        PacketReference::Ipv4(ipv4_packet) => match ipv4_packet.data {
                            IpPacketData::Udp(d) => {
                                if let Some(port) =
                                    UDP_PORTS_INCOMING.sync_lock().get(&d.header.destination)
                                {
                                    let d = d.to_owned();
                                    port.write(d).await;
                                }
                            }
                            _ => {
                                crate::VGA
                                    .print_str_async(&alloc::format!(
                                        "Received unhandled ip packet: {:02x?}\r\n",
                                        ipv4_packet
                                    ))
                                    .await;
                            }
                        },
                        PacketReference::Arp(address_resolution_protocol_packet) => {
                            crate::VGA
                                .print_str_async(&alloc::format!(
                                    "Received arp packet: {:02x?}\r\n",
                                    address_resolution_protocol_packet
                                ))
                                .await;
                            if address_resolution_protocol_packet.operation == 2 {
                                crate::VGA.print_str_async("Receive arp reply\r\n").await;
                                let mut table = IP_TO_MAC_TABLE.sync_lock();
                                if let Some(ip) =
                                    address_resolution_protocol_packet.get_sender_protocol_address()
                                {
                                    crate::VGA
                                        .print_str_async(&alloc::format!(
                                            "Processing arp reply for {}\r\n",
                                            ip
                                        ))
                                        .await;
                                    if let Some(mac) =
                                        address_resolution_protocol_packet.get_sender_mac_address()
                                    {
                                        crate::VGA
                                            .print_str_async("Processing arp reply\r\n")
                                            .await;
                                        table.insert(ip, Some(mac));
                                    }
                                }
                            } else if address_resolution_protocol_packet.operation == 1 {
                                doors_macros::todo_item!("Populate the actual ip address");
                                let myip = [11, 11, 11, 12];
                                if address_resolution_protocol_packet.target_protocol_address
                                    == myip
                                {
                                    let p = EthernetFrame {
                                        header: EthernetFrameHeader {
                                            destination: df.header.source,
                                            source: MacAddress::default(),
                                            vlan: None,
                                            ethertype: 2054,
                                        },
                                        contents: PacketReference::Arp(
                                            AddressResolutionProtocolPacket {
                                                htype: address_resolution_protocol_packet.htype,
                                                ptype: address_resolution_protocol_packet.ptype,
                                                address_length: address_resolution_protocol_packet
                                                    .address_length,
                                                protocol_length: address_resolution_protocol_packet
                                                    .protocol_length,
                                                operation: 2,
                                                sender_hardware_address: &mymac.address,
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
                                    todo!("Send the arp response");
                                }
                            }
                        }
                        PacketReference::Unknown(stuff) => {
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
            } else {
                crate::VGA.print_str("Received invalid ethernet frame\r\n");
            }
        }
        crate::executor::AsyncTask::yield_now().await;
    }
}

impl NetworkAdapter {
    /// Process network packets received
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    pub async fn run(mut self) {
        crate::VGA
            .print_str_async("Starting packet stuff for network interface\r\n")
            .await;
        let mymac = self.get_mac_address().await;
        crate::VGA.print_str_async("packet2\r\n").await;
        let r = self.get_receiver();
        crate::VGA.print_str_async("packet3\r\n").await;
        if let Some(r) = r {
            let s = self.get_sender();
            crate::executor::spawn(receive_packets(mymac, r, s)).unwrap();
        }
        crate::VGA.print_str_async("packet5\r\n").await;
        crate::executor::spawn(self.send_pending_packets()).unwrap();
        crate::VGA
            .print_str_async("Started packet stuff for network interface\r\n")
            .await;
    }
}

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

impl EthernetFrameHeader {
    /// Put the contents of the header into a packet
    pub fn add_to_packet(&self, p: &mut RawEthernetPacket) {
        assert!(p.length == 0);
        (&mut p.data[0..6]).copy_from_slice(&self.destination.address);
        (&mut p.data[6..12]).copy_from_slice(&self.source.address);
        let mut offset = 12;
        if let Some(q) = self.vlan {
            (&mut p.data[offset..offset + 4]).copy_from_slice(&q.to_be_bytes());
            offset += 4;
        }
        (&mut p.data[offset..offset + 2]).copy_from_slice(&self.ethertype.to_be_bytes());
        offset += 2;
        p.length = offset;
    }
}

/// A udp packet header
#[derive(Debug, Default)]
pub struct UdpPacketHeader {
    /// the source port
    source: u16,
    /// the destination port
    destination: u16,
    /// The length of the packet, including the 8 byte header
    length: u16,
    /// The packet checksum
    checksum: u16,
}

/// A udp packet
#[derive(Debug)]
pub struct UdpPacket {
    header: UdpPacketHeader,
    /// The actual packet data
    data: Box<[u8]>,
}

/// A udp packet
#[derive(Debug)]
pub struct UdpPacketReference<'a> {
    header: UdpPacketHeader,
    /// The actual packet data
    data: &'a [u8],
}

impl UdpPacketHeader {
    /// Send some data in an ethernet packet
    pub fn make_raw_packet(&self, rp: &mut RawEthernetPacket) {
        crate::VGA.print_str(&alloc::format!("ADDING UDP HEADER: {:?}\r\n", self));
        rp.data[rp.length..rp.length + 2].copy_from_slice(&self.source.to_be_bytes());
        rp.length += 2;
        rp.data[rp.length..rp.length + 2].copy_from_slice(&self.destination.to_be_bytes());
        rp.length += 2;
        rp.data[rp.length..rp.length + 2].copy_from_slice(&self.length.to_be_bytes());
        rp.length += 2;
        rp.data[rp.length..rp.length + 2].copy_from_slice(&self.checksum.to_be_bytes());
        rp.length += 2;
    }
}

impl<'a> UdpPacketReference<'a> {
    /// Send some data in an ethernet packet
    pub fn make_raw_packet(&self, rp: &mut RawEthernetPacket) {
        self.header.make_raw_packet(rp);
        rp.data[rp.length..rp.length + self.data.len()].copy_from_slice(&self.data);
        rp.length += self.data.len();
    }

    /// Convert to an owned packet
    pub fn to_owned(self) -> UdpPacket {
        UdpPacket {
            header: self.header,
            data: Box::from(self.data),
        }
    }
}

impl TryFrom<&[u8]> for UdpPacketHeader {
    type Error = ();
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            source: u16::from_be_bytes([value[0], value[1]]),
            destination: u16::from_be_bytes([value[2], value[3]]),
            length: u16::from_be_bytes([value[4], value[5]]),
            checksum: u16::from_be_bytes([value[6], value[7]]),
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for UdpPacketReference<'a> {
    type Error = ();
    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            header: UdpPacketHeader::try_from(value)?,
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
    Udp(UdpPacketReference<'a>),
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
#[derive(Debug, Default)]
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

impl Ipv4PacketHeader {
    /// Put the header into the given chunk of data
    pub fn construct_header_without_checksum(&self, d: &mut [u8]) {
        d[0] = (self.version << 4) | (self.header_size & 0xF);
        d[1] = (self.dscp << 2) | (self.ecn & 0x3);
        d[2..=3].copy_from_slice(&self.total_length.to_be_bytes());
        d[4..=5].copy_from_slice(&self.id.to_be_bytes());
        let b = ((self.flags as u16) << 13) | self.fragment_offset;
        d[6..=7].copy_from_slice(&b.to_be_bytes());
        d[8] = self.ttl;
        d[9] = self.protocol;
    }

    /// Add the header to the given packet
    pub fn add_to_packet(&mut self, rp: &mut RawEthernetPacket) {
        self.checksum = 0;
        let mut header = [0; 12];
        self.construct_header_without_checksum(&mut header);
        for b in &header {
            self.checksum = self.checksum.wrapping_add(*b as u16);
        }
        self.checksum ^= 0xffff;
        header[10..=11].copy_from_slice(&self.checksum.to_be_bytes());
        rp.data[rp.length..rp.length + 12].copy_from_slice(&header);
        rp.length += 12;
        rp.data[rp.length..rp.length + 4].copy_from_slice(&self.source.to_be_bytes());
        rp.length += 4;
        rp.data[rp.length..rp.length + 4].copy_from_slice(&self.destination.to_be_bytes());
        rp.length += 4;
        for a in self.options.iter().take(self.header_size as usize - 5) {
            for a in a.to_be_bytes() {
                rp.data[rp.length] = a;
                rp.length += 1;
            }
        }
    }
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
    /// The hardware type
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
    /// Get the ip address in ip format
    pub fn get_sender_protocol_address(&self) -> Option<core::net::IpAddr> {
        crate::VGA.print_str(&alloc::format!(
            "Processing arp reply for type {:x} {:x?}\r\n",
            self.ptype,
            self.sender_protocol_address
        ));
        if self.ptype == 0x0800 {
            let ip = [
                self.sender_protocol_address[0],
                self.sender_protocol_address[1],
                self.sender_protocol_address[2],
                self.sender_protocol_address[3],
            ];
            let a = core::net::IpAddr::from(ip);
            crate::VGA.print_str(&alloc::format!("Processing arp reply with {:?}\r\n", a));
            Some(a)
        } else {
            None
        }
    }

    /// Get the ethernet mac address of the sender
    pub fn get_sender_mac_address(&self) -> Option<MacAddress> {
        if self.htype == 1 {
            let mut address = [0; 6];
            address.copy_from_slice(&self.sender_hardware_address[0..6]);
            Some(MacAddress { address })
        } else {
            None
        }
    }

    /// Build the packet ontop of an ethernet layer
    pub async fn send_raw_packet(&self, layer: &EthernetLayer, rp: &mut RawEthernetPacket) {
        layer
            .send_raw_packet(
                0x0806,
                rp,
                |h| {},
                |rp| {
                    self.build_packet(rp);
                },
            )
            .await;
    }

    /// construct a packet in the specified raw packet
    fn build_packet(&self, rp: &mut RawEthernetPacket) {
        (&mut rp.data[rp.length..rp.length + 2]).copy_from_slice(&self.htype.to_be_bytes());
        rp.length += 2;
        (&mut rp.data[rp.length..rp.length + 2]).copy_from_slice(&self.ptype.to_be_bytes());
        rp.length += 2;
        rp.data[rp.length] = self.address_length;
        rp.length += 1;
        rp.data[rp.length] = self.protocol_length;
        rp.length += 1;
        (&mut rp.data[rp.length..rp.length + 2]).copy_from_slice(&self.operation.to_be_bytes());
        rp.length += 2;
        (&mut rp.data[rp.length..rp.length + self.address_length as usize])
            .copy_from_slice(&self.sender_hardware_address);
        rp.length += self.address_length as usize;
        (&mut rp.data[rp.length..rp.length + self.protocol_length as usize])
            .copy_from_slice(&self.sender_protocol_address);
        rp.length += self.protocol_length as usize;
        (&mut rp.data[rp.length..rp.length + self.address_length as usize])
            .copy_from_slice(&self.target_hardware_address);
        rp.length += self.address_length as usize;
        (&mut rp.data[rp.length..rp.length + self.protocol_length as usize])
            .copy_from_slice(&self.target_protocol_address);
        rp.length += self.protocol_length as usize;
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
pub enum PacketReference<'a> {
    /// An ip version 4 packet
    Ipv4(Ipv4Packet<'a>),
    /// Address resolution protocol
    Arp(AddressResolutionProtocolPacket<'a>),
    /// The packet type is unknown, but here is the data anyways, have fun!
    Unknown(&'a [u8]),
}

/// Represents a decoded ethernet frame
#[derive(Debug)]
pub struct EthernetFrame<'a> {
    /// The header
    header: EthernetFrameHeader,
    /// The packet contents
    contents: PacketReference<'a>,
}

/// Represents a received ethernet frame
#[derive(Debug)]
#[allow(unused)]
pub struct RawEthernetFrame<'a> {
    /// The header
    header: EthernetFrameHeader,
    /// The actual packet data
    data: Option<&'a [u8]>,
    /// The crc of the packet
    crc: u32,
}

impl<'a> TryFrom<&'a RawEthernetFrame<'a>> for EthernetFrame<'a> {
    type Error = ();
    fn try_from(value: &'a RawEthernetFrame<'a>) -> Result<Self, Self::Error> {
        if value.data.is_none() {
            return Err(());
        }
        let data = value.data.unwrap();
        Ok(match value.header.ethertype {
            0x0800 => Self {
                header: value.header.clone(),
                contents: PacketReference::Ipv4(Ipv4Packet::try_from(data)?),
            },
            0x0806 => Self {
                header: value.header.clone(),
                contents: PacketReference::Arp(data.into()),
            },
            _ => Self {
                header: value.header.clone(),
                contents: PacketReference::Unknown(data),
            },
        })
    }
}

/// A raw ethernet packet received from a network card
#[derive(Clone, Copy)]
pub struct RawEthernetPacket {
    /// The contents of the packet
    data: [u8; MAX_RX_PACKET_SIZE],
    /// The actual length of the packet
    length: usize,
}

impl RawEthernetPacket {
    /// Push some data to the packet
    pub fn push_data(&mut self, d: &[u8]) {
        let len = d.len();
        self.data[self.length..self.length + len].copy_from_slice(d);
        self.length += len;
    }
}

impl<'a> TryFrom<&'a RawEthernetPacket> for RawEthernetFrame<'a> {
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
            data: Some(dat),
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
    pub fn new_box() -> alloc::boxed::Box<Self> {
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

/// For sending and receiving udp frames
pub struct UdpLayer {
    ipv4: Ip4Layer,
    source: u16,
    destin: u16,
    /// The receiver side of packets sent to us
    recv: StreamReader<UdpPacket>,
}

impl UdpLayer {
    /// Construct a new layer for sending packets
    pub fn new(ipv4: Ip4Layer, source: u16, destin: u16) -> Self {
        let c = new_stream(5, 1);
        let mut u = UDP_PORTS_INCOMING.sync_lock();
        if !u.contains_key(&source) {
            u.insert(source, c.1);
        }
        Self {
            ipv4,
            source,
            destin,
            recv: c.0,
        }
    }

    /// Send the specified data in a udp packet
    pub async fn send_data(&self, d: &[u8]) {
        let mut header = UdpPacketHeader::default();
        let mut rp = RawEthernetPacket::new_box();
        let len = d.len() as u16;
        header.source = self.source;
        header.destination = self.destin;
        header.length = len + 8;
        self.ipv4
            .send_raw_packet(
                |ipv4| {
                    ipv4.protocol = 17;
                    len + 8
                },
                |rp| {
                    header.make_raw_packet(rp);
                    rp.push_data(d);
                },
                &mut rp,
            )
            .await;
    }

    /// Send some data in an ethernet packet
    pub async fn send_raw_packet<
        F: FnMut(&mut UdpPacketHeader) -> u16,
        G: FnMut(&mut RawEthernetPacket),
    >(
        &self,
        rp: &mut RawEthernetPacket,
        mut f: F,
        mut g: G,
    ) {
        let mut header = UdpPacketHeader::default();
        let len = f(&mut header);
        header.source = self.source;
        header.destination = self.destin;
        header.length = len + 8;
        self.ipv4
            .send_raw_packet(
                |ipv4| {
                    ipv4.protocol = 17;
                    len + 8
                },
                |rp| {
                    crate::VGA.print_str(&alloc::format!(
                        "PACKET IS CURRENT1 {} bytes\r\n",
                        rp.length
                    ));
                    header.make_raw_packet(rp);
                    crate::VGA.print_str(&alloc::format!(
                        "PACKET IS CURRENT2 {} bytes\r\n",
                        rp.length
                    ));
                    g(rp);
                    crate::VGA.print_str(&alloc::format!(
                        "PACKET IS CURRENT3 {} bytes\r\n",
                        rp.length
                    ));
                },
                rp,
            )
            .await;
    }
}

/// For sending and receiving ipv4 frames
pub struct Ip4Layer {
    ethernet: EthernetLayer,
    dest: core::net::IpAddr,
    src: core::net::IpAddr,
}

impl Ip4Layer {
    /// Build a new ipv4 layer
    pub fn new(ethernet: EthernetLayer, dest: core::net::IpAddr, src: core::net::IpAddr) -> Self {
        Self {
            ethernet,
            dest,
            src,
        }
    }

    /// Send some data in an ethernet packet
    pub async fn send_raw_packet<
        F: FnMut(&mut Ipv4PacketHeader) -> u16,
        G: FnMut(&mut RawEthernetPacket),
    >(
        &self,
        mut f: F,
        mut g: G,
        rp: &mut RawEthernetPacket,
    ) {
        let mut header = Ipv4PacketHeader::default();
        loop {
            {
                let mut mac_lookup = IP_TO_MAC_TABLE.sync_lock();
                let look_dst = !mac_lookup.contains_key(&self.dest);
                if look_dst {
                    crate::VGA.print_str("NEED TO GET MAC ADDRESS\r\n");
                    let arp_req = AddressResolutionProtocolPacket {
                        htype: 1,
                        ptype: 0x0800,
                        address_length: 6,
                        protocol_length: 4,
                        operation: 1,
                        sender_hardware_address: &self.ethernet.src.address,
                        sender_protocol_address: self.src.as_octets(),
                        target_hardware_address: &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
                        target_protocol_address: self.dest.as_octets(),
                    };
                    let mut rp2 = RawEthernetPacket::new_box();
                    mac_lookup.insert(self.dest, None);
                    arp_req.send_raw_packet(&self.ethernet, &mut rp2).await;
                }
                if let Some(Some(_)) = mac_lookup.get(&self.dest) {
                    break;
                }
            }
            crate::executor::AsyncTask::yield_now().await;
        }
        crate::VGA.print_str("GOT MAC ADDRESS\r\n");
        header.version = 4;
        header.header_size = 5;
        let i = self.src.as_octets();
        let srcip = [i[0], i[1], i[2], i[3]];
        header.source = u32::from_be_bytes(srcip);
        let i = self.dest.as_octets();
        let destip = [i[0], i[1], i[2], i[3]];
        header.destination = u32::from_be_bytes(destip);
        header.ttl = 40;
        header.total_length = f(&mut header) + header.header_size as u16 * 4;
        crate::VGA
            .print_str_async(&alloc::format!("Ip4 header {:?}\r\n", header))
            .await;
        self.ethernet
            .send_raw_packet(
                0x0800,
                rp,
                |p| {},
                |rp| {
                    crate::VGA.print_str(&alloc::format!(
                        "IP PACKET IS CURRENT1 {} bytes\r\n",
                        rp.length
                    ));
                    header.add_to_packet(rp);
                    crate::VGA.print_str(&alloc::format!(
                        "IP PACKET IS CURRENT2 {} bytes\r\n",
                        rp.length
                    ));
                    g(rp);
                    crate::VGA.print_str(&alloc::format!(
                        "IP PACKET IS CURRENT3 {} bytes\r\n",
                        rp.length
                    ));
                },
            )
            .await;
    }
}

/// For sending and receiving raw ethernet frames
pub struct EthernetLayer {
    dest: MacAddress,
    src: MacAddress,
    /// The sender to send packets with
    sender: IrqStreamWriter<RawEthernetPacket>,
}

impl EthernetLayer {
    /// Build a new ethernet layer
    pub fn new(
        dest: MacAddress,
        src: MacAddress,
        sender: IrqStreamWriter<RawEthernetPacket>,
    ) -> Self {
        Self { dest, src, sender }
    }

    /// Send some data in an ethernet packet after constructing it
    /// # Arguments
    /// * t: The ethertype value for the ethernet frame header
    /// * rp - The RawEthernetPacket to build the packet with
    /// * f - The closure used to optionally modify the ethernet frame header
    /// * g - The closure used to append the data to the packet
    pub async fn send_raw_packet<
        F: FnMut(&mut EthernetFrameHeader),
        G: FnMut(&mut RawEthernetPacket),
    >(
        &self,
        t: u16,
        rp: &mut RawEthernetPacket,
        mut f: F,
        mut g: G,
    ) {
        let mut header = EthernetFrameHeader {
            destination: self.dest,
            source: self.src,
            vlan: None,
            ethertype: t,
        };
        f(&mut header);
        header.add_to_packet(rp);
        g(rp);
        {
            let crc_calc = crc::Crc::<u32>::new(&crc::CRC_32_BZIP2);
            let c = crc_calc.checksum(&rp.data[0..rp.length]);
            let c = c.to_be_bytes();
            rp.data[rp.length..rp.length + 4].copy_from_slice(&c[0..4]);
            rp.length += 4;
        }
        self.sender.write(*rp).await;
        crate::VGA.print_str("QUEUED A PACKET TO SEND\r\n");
    }
}

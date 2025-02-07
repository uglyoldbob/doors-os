//! This driver is for the intel pro/1000 networking hardware.
//! TODO: Implement support notation for ipv6 (82544GC/EI does not support ipv6)

use alloc::collections::btree_map::BTreeMap;

use crate::modules::network::{MacAddress, NetworkAdapterTrait};
use crate::modules::video::TextDisplayTrait;
use crate::modules::{
    pci::{
        BarSpace, ConfigurationSpaceEnum, PciBus, PciConfigurationSpace, PciDevice, PciFunction,
        PciFunctionDriver, PciFunctionDriverTrait,
    },
    video::{hex_dump, hex_dump_generic},
};
use crate::IoReadWrite;

/// Holds either memory or io space
enum MemoryOrIo {
    /// Regular memory
    Memory(crate::PciMemory),
    /// Io space
    Io(crate::IoPortArray<'static>),
}

/// The model variants for the pro1000
#[derive(Debug)]
enum Model {
    /// TODO
    Model82540EP_A_Desktop,
    /// TODO
    Model82540EP_A_Mobile,
    /// TODO
    Model82540EM_A_Desktop,
    /// TODO
    Model82540EM_A_Mobile,
    /// TODO
    Model82541EI_A0_or_Model82541EI_B0_Copper,
    /// TODO
    Model82541EI_B0_Mobile,
    /// TODO
    Model82541GI_B1_Copper_or_Model82541PI_C0,
    /// TODO
    Model82541GI_B1_Mobile,
    /// TODO
    Model82541PI_C0,
    /// TODO
    Model82544EI_A4,
    /// TODO
    Model82544GC_A4,
    /// TODO
    Model82545EM_A_Copper,
    /// TODO
    Model82545EM_A_Fiber,
    /// TODO
    Model82545GM_B_Copper,
    /// TODO
    Model82545GM_B_Fiber,
    /// TODO
    Model82545GM_B_SerDes,
    /// TODO
    Model82546EB_A1_CopperDual,
    /// TODO
    Model82546EB_A1_Fiber,
    /// TODO
    Model82546EB_A1_CopperQuad,
    /// TODO
    Model82546GB_B0_Copper,
    /// TODO
    Model82546GB_B0_Fiber,
    /// TODO
    Model82546GB_B0_SerDes,
    /// TODO
    Model82547EI_A0_or_Model82547EI_A1_or_Model82547EI_B0_Copper_or_Model82547GI_B0,
    /// TODO
    Model82547EI_B0_Mobile,
}

impl TryFrom<u16> for Model {
    type Error = ();
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0x100e => Ok(Self::Model82540EM_A_Desktop),
            0x100f => Ok(Self::Model82545EM_A_Copper),
            0x1011 => Ok(Self::Model82545EM_A_Fiber),
            0x1015 => Ok(Self::Model82540EM_A_Mobile),
            0x1019 => Ok(Self::Model82547EI_A0_or_Model82547EI_A1_or_Model82547EI_B0_Copper_or_Model82547GI_B0),
            0x101a => Ok(Self::Model82547EI_B0_Mobile),
            0x1010 => Ok(Self::Model82546EB_A1_CopperDual),
            0x1012 => Ok(Self::Model82546EB_A1_Fiber),
            0x1013 => Ok(Self::Model82541EI_A0_or_Model82541EI_B0_Copper),
            0x1016 => Ok(Self::Model82540EP_A_Mobile),
            0x1017 => Ok(Self::Model82540EP_A_Desktop),
            0x1018 => Ok(Self::Model82541EI_B0_Mobile),
            0x101d => Ok(Self::Model82546EB_A1_CopperQuad),
            0x1026 => Ok(Self::Model82545GM_B_Copper),
            0x1027 => Ok(Self::Model82545GM_B_Fiber),
            0x1028 => Ok(Self::Model82545GM_B_SerDes),
            0x1076 => Ok(Self::Model82541GI_B1_Copper_or_Model82541PI_C0),
            0x1077 => Ok(Self::Model82541GI_B1_Mobile),
            0x1078 => Ok(Self::Model82541PI_C0),
            0x1079 => Ok(Self::Model82546GB_B0_Copper),
            0x107a => Ok(Self::Model82546GB_B0_Fiber),
            0x107b => Ok(Self::Model82546GB_B0_SerDes),
            0x1107 => Ok(Self::Model82544EI_A4),
            0x1112 => Ok(Self::Model82544GC_A4),
            _ => Err(()),
        }
    }
}

impl MemoryOrIo {
    /// Dump the contents of the data as hex
    fn hex_dump(&self) {
        match self {
            MemoryOrIo::Memory(_m) => {
                let mut buffer = [0u32; 32];
                for (i, b) in buffer.iter_mut().enumerate() {
                    *b = self.read(i as u16);
                }
                hex_dump_generic(&buffer, true, false);
            }
            MemoryOrIo::Io(_io_port_array) => todo!(),
        }
    }

    /// Read a u32 from the specified address
    fn read(&self, address: u16) -> u32 {
        match self {
            MemoryOrIo::Memory(mem) => mem.read_u32(address as usize),
            MemoryOrIo::Io(io) => {
                let mut iop: crate::IoPortRef<u32> = io.port(address);
                iop.port_read()
            }
        }
    }

    /// Write the specified address with the specified u32
    fn write(&mut self, address: u16, val: u32) {
        match self {
            MemoryOrIo::Memory(mem) => {
                mem.write_u32(address as usize, val);
            }
            MemoryOrIo::Io(io) => {
                let mut iop: crate::IoPortRef<u32> = io.port(address);
                iop.port_write(val);
            }
        }
    }
}

/// Defines the addresses of various registers for the pro1000 device
#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u16)]
enum IntelPro1000Registers {
    /// The eeprom read register
    Eeprom = 0x14,
    /// Receive control register
    Rctrl = 0x100,
    /// Transmit control register
    Tctrl = 0x400,
    /// Receive descriptor base low
    RxDescLow = 0x2800,
    /// Receive descriptor base high
    RxDescHigh = 0x2804,
    /// Receive descriptor length
    RxDescLen = 0x2808,
    /// Receive descriptor head
    RxDescHead = 0x2810,
    /// Receive descriptor tail
    RxDescTail = 0x2818,
    /// Transmit descriptor base low
    TxDescLow = 0x3800,
    /// Transmit descriptor base high
    TxDescHigh = 0x3804,
    /// Transmit descriptor length
    TxDescLen = 0x3808,
    /// Transmit descriptor head
    TxDescHead = 0x3810,
    /// Transmit descriptor tail
    TxDescTail = 0x3818,
}

/// Ethernet driver for the intel pro/1000 ethernet controller on pci
#[derive(Clone, Default)]
pub struct IntelPro1000 {}

bitfield::bitfield! {
    /// The status of an rx descriptor
    struct RxBufferStatus(u8);
    impl Debug;
    impl new;
    /// the hardware is done with the descriptor
    dd, _ : 0;
    /// end of packet, last descriptor for an incoming packet
    eop, _ : 1;
    /// ignore checksum indication, ignore the checksum indicators when set
    ixsm, _ : 2;
    /// The packet is 802.1q. The packet type matches VET. Only set when CTRL.VME is set.
    vp, _ : 3;
    /// checksum was performed
    tcpcs, _ : 5;
    /// ip checksum on packet was calculated by hardware
    ipcs, _ : 6;
    /// passed in-exact filter. Used to expedite processing that determines if a packet is for this station
    pif, _ : 7;
}

bitfield::bitfield! {
    /// The error field of an rx descriptor
    struct RxError(u8);
    impl Debug;
    impl new;
    /// crc error or alignment error, check statistics registers to distinguish between the two
    ce, _ : 0;
    /// symbol error. packet received with bad symbol. Only for TBI / SerDes mode.
    se, _ : 1;
    /// sequence error. received packet contained a bad delimiter sequence (TBI or SerDes mode). for 802.3 this is a framing error.
    /// Valid sequence is as follows: idle, start of frame, data, Option<pad>, end of frame, Option<fill>, idle.
    seq, _ : 2;
    /// carrier extension error. GMII interface indicates a carrier extension error. Only valid for 1000Mbps half-duplex operations. Only valid for the 82544GC/EI models.
    cxe, _ : 4;
    /// tcp/udp checksum error. Only valid when status.tcpcs is set.
    tcpe, _ : 5;
    /// ip checksum error. Only valid when status.ipcs is set.
    ipe, _ : 6;
    /// rx data error. error during packet reception. For TBI / internal SerDes mode, a /V/ code was received. For MII or GMII mode, i_RX_ER was adderted during packet reception. Only valid when status.eop and status.dd are set. Only set when rctl.sbp is set.
    rxe, _ : 7;
}

bitfield::bitfield! {
    /// The special field of an rx descriptor. For storing additional information of 802.1q packets. (not valid for model 82544GC/EI).
    struct RxSpecial(u16);
    impl Debug;
    impl new;
    /// vlan identifier
    vlan, _ : 11, 0;
    /// canonical form indicator
    cfi, _ : 12;
    /// user priority
    pri, _ : 15, 13;
}

/// An Rx buffer for the device
#[repr(C, packed)]
struct RxBuffer {
    /// the physical address of the buffer
    address: u64,
    /// packet length
    length: u16,
    /// the checksum of the packet (not valid for model 82544GC/EI)
    checksum: u16,
    /// descriptor status
    status: RxBufferStatus,
    /// receive errors, TODO make a bitfield for this
    errors: RxError,
    /// extra data for 802.1q packets (not valid for model 82544GC/EI)
    special: RxSpecial,
}

impl RxBuffer {
    /// Construct a new [Self]. address must be the physical address
    fn new(address: u64) -> Self {
        Self {
            address,
            length: 0,
            checksum: 0,
            status: RxBufferStatus::new(),
            errors: RxError::new(),
            special: RxSpecial::new(),
        }
    }
}

/// A TxBuffer for the device
#[repr(C, packed)]
struct TxBuffer {
    /// TODO
    address: u64,
    /// TODO
    length: u16,
    /// TODO
    cso: u8,
    /// TODO
    cmd: u8,
    /// TODO
    status: u8,
    /// TODO
    css: u8,
    /// TODO
    special: u16,
}

impl TxBuffer {
    /// Construct a new [Self], address must be the physical address
    fn new(address: u64) -> Self {
        Self {
            address,
            length: 0,
            cso: 0,
            cmd: 0,
            status: 0,
            css: 1,
            special: 0,
        }
    }
}

/// Holds all information required for the multiple rx buffers required for the network card
struct RxBuffers {
    /// The structures used by the network card
    bufs: crate::DmaMemorySlice<RxBuffer>,
    /// The structures used to manage the buffers
    dmas: alloc::vec::Vec<crate::DmaMemorySlice<u8>>,
}

impl RxBuffers {
    /// Construct a new set of tx buffers of the specified quantity and size in bytes
    fn new(quantity: u8, size: usize) -> Result<Self, core::alloc::AllocError> {
        let mut dmas = alloc::vec::Vec::with_capacity(quantity as usize);
        for _i in 0..quantity {
            dmas.push(crate::DmaMemorySlice::new(size)?);
        }
        let bufs = crate::DmaMemorySlice::new_with(quantity as usize, |i| {
            let dma = &dmas[i];
            Ok(RxBuffer::new(dma.phys() as u64))
        })?;
        Ok(Self { bufs, dmas })
    }
}

/// Holds all information required for the multiple tx buffers required for the network card
struct TxBuffers {
    /// The structures used by the network card
    bufs: crate::DmaMemorySlice<TxBuffer>,
    /// The structures used to manage the buffers
    dmas: alloc::vec::Vec<crate::DmaMemorySlice<u8>>,
}

impl TxBuffers {
    /// construct a new set of tx buffers of the specified quantity and size in bytes
    fn new(quantity: u8, size: usize) -> Result<Self, core::alloc::AllocError> {
        let mut dmas = alloc::vec::Vec::with_capacity(quantity as usize);
        for _i in 0..quantity {
            dmas.push(crate::DmaMemorySlice::new(size)?);
        }
        let bufs = crate::DmaMemorySlice::new_with(quantity as usize, |i| {
            let dma = &dmas[i];
            Ok(TxBuffer::new(dma.phys() as u64))
        })?;
        Ok(Self { bufs, dmas })
    }
}

#[doors_macros::enum_variant(NetworkAdapter)]
/// The actual intel pro/1000 device
pub struct IntelPro1000Device {
    /// The base address registers
    _bars: [Option<BarSpace>; 6],
    /// The memory allocated by bar0
    bar0: MemoryOrIo,
    /// the io space allocated for the device
    _io: crate::IoPortArray<'static>,
    /// Is the eeprom present?
    eeprom_present: Option<bool>,
    /// The rx buffers
    rxbufs: Option<RxBuffers>,
    /// The current rx buffer
    rxbufindex: Option<u8>,
    /// The tx buffers
    txbufs: Option<TxBuffers>,
    /// The current tx buffer
    txbufindex: Option<u8>,
    /// The specific model of the device
    model: Model,
}

bitflags::bitflags! {
    /// Represents a set of flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct RctrlFlags: u32 {
        /// Receiver enable. After disabling the receiver, reset the receiver before enabling it.
        const EN = 1<<1;
        /// Store bad packets. Store bad packets (crc error, symbol error, sequence error, length error, alignment error, short packets, carrier extension or RX_ERR) that pass the filter function.
        const SBP = 1<<2;
        /// Unicast promiscuous mode enabled. Passes all received unicast packets without filtering them.
        const UPE = 1<<3;
        /// Multicast promiscuous mode enabled. Passes all received multicast packets without filtering them.
        const MPE = 1<<4;
        /// Long packet reception enable. Allows packets with a length of up to 16384 bytes when set, otherwise allows packets of length 1522 bytes when not set.
        const LPE = 1<<5;
        /// Loopback enabled. Only allowed for ful-duplex operations. Not supported by 82540EP/EM, 82541XX, and 82547GI/EI models.
        const LOOPBACK = 3<<6;
        /// Set the receive descriptor minimum threshold size to 1/2 of RDLEN
        const RDMTS_HALF = 0;
        /// Set the receive descriptor minimum threshold size to 1/4 of RDLEN
        const RDMTS_QUARTER = 1<<8;
        /// Set the receive descriptor minimum threshold size to 1/8 of RDLEN
        const RDMTS_EIGHTH = 2<<8;
        /// Multicast offset use bits [47:36]
        const MO_36 = 0;
        /// Multicast offset use bits [46:35]
        const MO_35 = 1<<12;
        /// Multicast offset use bits [45:34]
        const MO_34 = 2<<12;
        /// Multicast offset use bits [43:32]
        const MO_32 = 3<<12;
        /// Broadcast acept mode
        const BAM = 1<<15;
        /// Receive buffer size 16384 bytes
        const BSIZE_16384 = 1<<16 | 1<<25;
        /// Receive buffer size 8192 bytes
        const BSIZE_8192 = 2<<16 | 1<<25;
        /// Receive buffer size 4096 bytes
        const BSIZE_4096 = 3<<16 | 1<<25;
        /// Receive buffer size 2048 bytes
        const BSIZE_2048 = 0;
        /// Receive buffer size 1024 bytes
        const BSIZE_1024 = 1<<16;
        /// Receive buffer size 512 bytes
        const BSIZE_512 = 2<<16;
        /// Receive buffer size 256 bytes
        const BSIZE_256 = 3<<16;
        /// vlan filter enable. See also CFIEN, CFI
        const VFE = 1<<18;
        /// canonical form indicator enable for 802.1q packets.
        const CFIEN = 1<<19;
        /// canonical form indicator bit value. When CFIEN is set, packets with CFI equal to this field are accepted.
        const CFI = 1<<20;
        /// discard pause frames
        const DPF = 1<<22;
        /// pass MAC control frames
        const PMCF = 1<<23;
        /// strip ethernet CRC from incoming packet
        const SECRC = 1<<26;
    }
}

bitflags::bitflags! {
    /// Represents a set of flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct TctrlFlags: u32 {
        /// Transmit enable. After disabling, the transmitter should be reset before enabling.
        const EN = 1<<1;
        /// pad short packets enable. Makes short packets 64 bytes long by padding with data bytes, otherwise the minimum packet length is 32 bytes. Not the same as minimum collision distance.
        const PSP = 1<<3;
        /// collision threshold shift.
        /// # Examples
        /// ```
        /// let ct = 14<<TctrlFlags::CT_SHIFT.bits();
        /// ```
        const CT_SHIFT = 4;
        /// collision threshold base mask
        const CT_BASE_MASK = 0xff;
        /// collision threshold final mask
        const CT_FINAL_MASK = Self::CT_BASE_MASK.bits()<<Self::CT_SHIFT.bits();
        /// collision distance. Minimum number of byte times to elapse for proper CSMA/CD operation. Packets are padded with special symbols.
        /// # Examples
        /// ```
        /// let cold = 14<<TctrlFlags::COLD_SHIFT.bits();
        /// ```
        const COLD_SHIFT = 12;
        /// the collision distance base mask
        const COLD_BASE_MASK = 0x3FF;
        /// The collision distance final mask
        const COLD_FINAL_MASK = Self::COLD_BASE_MASK.bits()<<Self::COLD_SHIFT.bits();
        /// software XOFF transmission. schedules the transmission of an XOFF (puase) frame using the current value of the PAUSe timer (FCTTV.TTV)
        const SWXOFF = 1<<22;
        /// retransmit on late colision. Enables retransmit when there is a late collision event. Collision window is speed dependent. (64 bytes for 10/100 Mbps, 512 bytes for 1000 Mbps). Only for half-duplex mode.
        const RTLC = 1<<24;
        /// No re-transmit on underrun (8244GC/EI only)
        const NRTU = 1<<25;
    }
}

impl super::super::NetworkAdapterTrait for IntelPro1000Device {
    fn get_mac_address(&mut self) -> MacAddress {
        if self.detect_eeprom() {
            let v = self.read_from_eeprom(0);
            let v2 = self.read_from_eeprom(1);
            let v3 = self.read_from_eeprom(2);
            let v = v.to_le_bytes();
            let v2 = v2.to_le_bytes();
            let v3 = v3.to_le_bytes();
            MacAddress {
                address: [v[0], v[1], v2[0], v2[1], v3[0], v3[1]],
            }
        } else {
            todo!();
        }
    }
}

impl IntelPro1000Device {
    /// Detect the presence of an eeprom and store the result
    fn detect_eeprom(&mut self) -> bool {
        if self.eeprom_present.is_none() {
            self.bar0.write(IntelPro1000Registers::Eeprom as u16, 1);
            self.eeprom_present = Some(false);
            for _i in 0..10000 {
                let val = self.bar0.read(IntelPro1000Registers::Eeprom as u16);
                let val2 = val & 0x10;
                doors_macros2::kernel_print!("EEPROM DETECT: {:x} {:x}\r\n", val, val2);
                if (val2) != 0 {
                    self.eeprom_present = Some(true);
                    break;
                }
            }
        }
        self.eeprom_present.unwrap()
    }

    /// Does the device support pci-x extension to pci?
    fn supports_pcix(&self) -> bool {
        !matches!(
            self.model,
            Model::Model82541EI_A0_or_Model82541EI_B0_Copper
                | Model::Model82541EI_B0_Mobile
                | Model::Model82541GI_B1_Copper_or_Model82541PI_C0
                | Model::Model82541GI_B1_Mobile
                | Model::Model82541PI_C0
                | Model::Model82540EP_A_Desktop
                | Model::Model82540EP_A_Mobile
                | Model::Model82540EM_A_Desktop
                | Model::Model82540EM_A_Mobile
        )
    }

    /// Initialize the rx buffers for the device
    fn init_rx(&mut self, mac: &MacAddress) -> Result<(), core::alloc::AllocError> {
        if self.rxbufs.is_none() {
            let rxbuf = RxBuffers::new(32, 8192)?;
            let rxaddr = rxbuf.bufs.phys();
            doors_macros2::kernel_print!("Writing RX stuff to network card\r\n");
            self.bar0.write(
                IntelPro1000Registers::RxDescLow as u16,
                (rxaddr >> 32) as u32,
            );
            self.bar0.write(
                IntelPro1000Registers::RxDescHigh as u16,
                (rxaddr & 0xFFFFFFFF) as u32,
            );
            self.bar0.write(
                IntelPro1000Registers::RxDescLen as u16,
                core::mem::size_of::<RxBuffer>() as u32 * rxbuf.bufs.len() as u32,
            );
            self.bar0.write(IntelPro1000Registers::RxDescHead as u16, 0);
            self.bar0.write(
                IntelPro1000Registers::RxDescTail as u16,
                rxbuf.bufs.len() as u32 - 1,
            );
            self.bar0.write(
                IntelPro1000Registers::Rctrl as u16,
                (RctrlFlags::EN | RctrlFlags::RDMTS_HALF | RctrlFlags::BSIZE_8192).bits(),
            );
            self.rxbufindex = Some(0);
            doors_macros2::kernel_print!(
                "RX BUFFER ARRAY IS AT virtual {:x} physical {:x}, size {}\r\n",
                rxbuf.bufs.virt(),
                rxaddr,
                rxbuf.bufs.size()
            );
            for r in rxbuf.dmas.iter() {
                doors_macros2::kernel_print!(
                    "\tIndividual buffer addr is virtual {:x} physical {:x}, size {}\r\n",
                    r.virt(),
                    r.phys(),
                    r.size()
                );
            }
            self.rxbufs = Some(rxbuf);
        }
        Ok(())
    }

    /// Initialize the tx buffers for the device
    fn init_tx(&mut self) -> Result<(), core::alloc::AllocError> {
        if self.txbufs.is_none() {
            let txbuf = TxBuffers::new(8, 8192)?;
            let txaddr = txbuf.bufs.phys();
            self.bar0.write(
                IntelPro1000Registers::TxDescLow as u16,
                (txaddr >> 32) as u32,
            );
            self.bar0.write(
                IntelPro1000Registers::TxDescHigh as u16,
                (txaddr & 0xFFFFFFFF) as u32,
            );
            self.bar0.write(
                IntelPro1000Registers::TxDescLen as u16,
                core::mem::size_of::<TxBuffer>() as u32 * txbuf.bufs.len() as u32,
            );
            self.bar0.write(IntelPro1000Registers::TxDescHead as u16, 0);
            self.bar0.write(IntelPro1000Registers::TxDescTail as u16, 0);
            self.bar0.write(
                IntelPro1000Registers::Tctrl as u16,
                (TctrlFlags::EN | TctrlFlags::PSP | TctrlFlags::RTLC).bits()
                    | (15 << TctrlFlags::CT_SHIFT.bits())
                    | (64 << TctrlFlags::COLD_SHIFT.bits()),
            );
            self.txbufindex = Some(0);
            doors_macros2::kernel_print!(
                "TX BUFFER ARRAY IS AT virtual {:x} physical {:x}, size {}\r\n",
                txbuf.bufs.virt(),
                txaddr,
                txbuf.bufs.size()
            );
            for r in txbuf.dmas.iter() {
                doors_macros2::kernel_print!(
                    "\tIndividual buffer addr is virtual {:x} physical {:x}, size {}\r\n",
                    r.virt(),
                    r.phys(),
                    r.size()
                );
            }
            self.txbufs = Some(txbuf);
        }
        Ok(())
    }

    /// Read a word from the eeprom at the specified address
    fn read_from_eeprom(&mut self, addr: u8) -> u16 {
        if self.detect_eeprom() {
            self.bar0.write(
                IntelPro1000Registers::Eeprom as u16,
                1 | ((addr as u32) << 8),
            );
            loop {
                let a = self.bar0.read(IntelPro1000Registers::Eeprom as u16);
                if (a & (0x10)) != 0 {
                    return (a >> 16) as u16;
                } else {
                    //doors_macros2::kernel_print!("VAL1: {:x}\r\n", a);
                }
            }
        } else {
            self.bar0.write(
                IntelPro1000Registers::Eeprom as u16,
                1 | ((addr as u32) << 2),
            );
            loop {
                let a = self.bar0.read(IntelPro1000Registers::Eeprom as u16);
                if (a & (0x2)) != 0 {
                    return (a >> 16) as u16;
                } else {
                    //doors_macros2::kernel_print!("VAL2: {:x}\r\n", a);
                }
            }
        }
    }
}

impl IntelPro1000 {
    /// Create a new self, in const form
    pub const fn new() -> Self {
        Self {}
    }
}

impl PciFunctionDriverTrait for IntelPro1000 {
    fn register(&self, m: &mut BTreeMap<u32, PciFunctionDriver>) {
        doors_macros2::kernel_print!("Register intel pro/1000 pci driver\r\n");
        for dev in [
            0x100e, 0x100f, 0x1011, 0x1015, 0x1019, 0x101a, 0x1010, 0x1012, 0x1013, 0x1016, 0x1017,
            0x1018, 0x101d, 0x1026, 0x1027, 0x1028, 0x1076, 0x1077, 0x1078, 0x1079, 0x107a, 0x107b,
            0x1107, 0x1112,
        ] {
            let dev = dev as u16;
            let vendor_combo = ((dev as u32) << 16) | 0x8086;
            m.entry(vendor_combo).or_insert_with(|| self.clone().into());
        }
    }

    fn parse_bars(
        &mut self,
        cs: &mut PciConfigurationSpace,
        bus: &PciBus,
        dev: &PciDevice,
        f: &PciFunction,
        config: &ConfigurationSpaceEnum,
        mut bars: [Option<BarSpace>; 6],
    ) {
        let bar0 = {
            if let Some(bar) = &mut bars[0] {
                if bar.is_size_valid() {
                    doors_macros2::kernel_print!("PCI PARSE BAR {}\r\n", bar.get_index());
                    bar.print();
                    let d = bar.get_memory(cs, bus, dev, f, config);
                    if let Some(d) = d {
                        doors_macros2::kernel_print!("Got memory at {:x}\r\n", d.virt());
                        Some(MemoryOrIo::Memory(d))
                    } else {
                        bar.get_io(cs, bus, dev, f, config).map(MemoryOrIo::Io)
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };
        let io = bars.iter_mut().find_map(|a| {
            if let Some(a) = a {
                a.get_io(cs, bus, dev, f, config)
            } else {
                None
            }
        });
        let configspace = f.get_all_configuration(cs, bus, dev);
        configspace.dump("\t");
        if let Some(m) = bar0 {
            if let Some(i) = io {
                for b in bars.iter().flatten() {
                    b.print();
                }
                let model = Model::try_from(configspace.get_device_id()).unwrap();
                let mut d = IntelPro1000Device {
                    _bars: bars,
                    bar0: m,
                    _io: i,
                    eeprom_present: None,
                    rxbufs: None,
                    rxbufindex: None,
                    txbufs: None,
                    txbufindex: None,
                    model,
                };
                d.bar0.hex_dump();
                doors_macros2::kernel_print!("Detected model as {:?}\r\n", d.model);
                doors_macros2::kernel_print!("EEPROM DETECTED: {}\r\n", d.detect_eeprom());
                let mut data = [0u16; 256];
                for (i, data) in data.iter_mut().enumerate() {
                    *data = d.read_from_eeprom(i as u8);
                }
                hex_dump_generic(&data, true, false);
                let mac = d.get_mac_address();
                hex_dump(&mac.address, false, false);
                if let Err(e) = d.init_rx(&mac) {
                    doors_macros2::kernel_print!("RX buffer allocation error {:?}\r\n", e);
                }
                if let Err(e) = d.init_tx() {
                    doors_macros2::kernel_print!("TX buffer allocation error {:?}\r\n", e);
                }
                super::super::register_network_adapter(d.into());
            }
        }
    }
}

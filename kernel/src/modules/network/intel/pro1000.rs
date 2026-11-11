//! This driver is for the intel pro/1000 networking hardware.
//! TODO: Implement support notation for ipv6 (82544GC/EI does not support ipv6)
//!
//! # Descriptor rings
//!
//! Both rings are circular and the ring is empty when head == tail. That means the tail pointer
//! always names the descriptor *after* the last one that has been made available, so one
//! descriptor in each ring is a spare that keeps "full" and "empty" distinguishable. Every place
//! that writes a head or tail pointer in this driver follows that single convention:
//!
//! * The receive ring is started with head 0 and tail `RX_DESC_COUNT - 1`, so hardware may write
//!   descriptors `0 ..= RX_DESC_COUNT - 2`.
//! * After the driver consumes descriptor `i` it clears the descriptor and writes tail `i + 1`.
//! * After preparing transmit descriptor `i` the driver writes tail `i + 1`.
//!
//! # Memory shared with the device
//!
//! The device is given base addresses and lengths and then reads and writes those ranges without
//! any knowledge of the page table, so every descriptor and packet buffer lives in a
//! [crate::DmaPage] (one physically contiguous page frame). Memory from the regular heap must
//! never be handed to the device: each page of a heap allocation is mapped to an independently
//! allocated page frame, so only the first page of such a region is where the device thinks it is.
//!
//! Anything the device writes is read with [core::ptr::read_volatile] through a raw pointer, and
//! anything the driver writes that the device will read is followed by a compiler fence before the
//! notifying register write. Long lived references into device memory would let the compiler cache
//! values that the device changes underneath it.
//!
//! # Interrupts and locks
//!
//! The interrupt handler only uses [crate::IrqGuarded::interrupt_access], pushes packets onto a
//! stream, and never prints. The asynchronous side acquires `bar0` before `rxbufs` when it needs
//! both, matching the handler, and does the register programming before taking the ring lock so
//! that no two guards are held at once.

use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::format;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use futures::StreamExt;

use crate::kernel::System;
use crate::modules::network::{MacAddress, NetworkAdapterTrait};
use crate::modules::pci::PciFunctionDriverTrait;
use crate::modules::pci::{
    BarSpace, ConfigurationSpaceEnum, PciBus, PciConfigurationSpace, PciDevice, PciFunction,
};
use crate::modules::PciFunctionDriver;
use crate::{
    new_irq_stream, Arc, DmaPage, IoReadWrite, IrqGuarded, IrqGuardedInner, IrqNumbers,
    IrqStreamReader, IrqStreamWriter, Locked,
};

/// Log a message only when extra kernel logging is enabled.
///
/// Only usable from async functions, and only outside of interrupt context.
macro_rules! net_print {
    ($($arg:tt)*) => {
        if crate::DEBUG_PRINT.load(core::sync::atomic::Ordering::SeqCst) {
            crate::VGA.print_str_async(&format!($($arg)*)).await;
        }
    };
}

/// The number of receive descriptors. Each one gets its own packet buffer.
const RX_DESC_COUNT: usize = 32;
/// The number of transmit descriptors.
const TX_DESC_COUNT: usize = 8;
/// The size of a single packet buffer. Must match the buffer size programmed into RCTL, and must
/// fit in one page since each buffer is backed by exactly one page.
const PACKET_BUFFER_SIZE: usize = 4096;
/// The receive buffer size selection matching [PACKET_BUFFER_SIZE].
const RCTL_BUFFER_SIZE: RctrlFlags = RctrlFlags::BSIZE_4096;

/// Descriptor status: the device is done with this descriptor.
const DESC_STATUS_DD: u8 = 1;
/// Inter packet gap value used for copper phy models, the power on default of the register.
const TIPG_COPPER: u32 = 0x6007;
/// Interrupt throttling rate in 256ns units, roughly one millisecond.
const ITR_1MS: u32 = 4000;
/// The number of causes read from the interrupt cause register in one interrupt.
const MAX_INTERRUPT_PASSES: usize = 4;
/// How long to wait for the interrupt mask register to accept the requested mask.
const IMS_VERIFY_TRIES: u32 = 100;
/// How long to wait for an mdi transaction to complete.
const MDIC_WAIT_TRIES: u32 = 10_000;
/// How long to wait for an eeprom transaction to complete.
const EEPROM_WAIT_TRIES: u32 = 10_000;
/// How long to wait for the device to write back a transmit descriptor.
const TX_COMPLETE_TRIES: u32 = 1_000_000;
/// Approximate delays expressed as yield counts.
///
/// The kernel's millisecond delay helpers panic when no arbitrary timing timer is available, which
/// is not acceptable during device probing, and every datasheet delay in this driver is either
/// followed by a check of the condition the delay exists to guarantee or is only a minimum hold
/// time. The counts are chosen generously so that the real delays are comfortably exceeded.
const RESET_SETTLE_YIELDS: u32 = 20_000;
/// As [RESET_SETTLE_YIELDS], for the phy reset hold time.
const PHY_RESET_HOLD_YIELDS: u32 = 20_000;
/// How many times to poll for the link before giving up on a transmission.
const LINK_POLLS: u32 = 100;
/// Yields between link polls.
const LINK_POLL_YIELDS: u32 = 1_000;

/// Yield the executor a number of times. See [RESET_SETTLE_YIELDS].
async fn yield_delay(count: u32) {
    for _ in 0..count {
        crate::executor::AsyncTask::yield_now().await;
    }
}

/// Allocate a page for the device to use, rejecting memory the device cannot reach.
///
/// The pro/1000 family masters the bus with 32 bit addresses.
fn dma_page() -> Result<DmaPage, core::alloc::AllocError> {
    let page = DmaPage::new()?;
    if (page.phys() as u64) > u32::MAX as u64 {
        // The page is returned to the allocator when it is dropped here.
        return Err(core::alloc::AllocError);
    }
    Ok(page)
}

/// Holds either memory or io space
enum MemoryOrIo {
    /// Regular memory
    Memory(crate::PciMemory),
    /// Io space
    Io(crate::IoPortArray<'static>),
}

/// The model variants for the pro1000
///
/// Note that the 82540xx and 82541xx models do not implement the pci-x extensions; the driver does
/// not currently need to distinguish them.
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
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

/// The device ids the driver claims, paired with the model each one is. This is the single source
/// of truth: both the pci driver registration and the model lookup read it, so adding a device id
/// is a one line change.
const MODELS: &[(u16, Model)] = &[
    (0x100e, Model::Model82540EM_A_Desktop),
    (0x100f, Model::Model82545EM_A_Copper),
    (0x1011, Model::Model82545EM_A_Fiber),
    (0x1015, Model::Model82540EM_A_Mobile),
    (
        0x1019,
        Model::Model82547EI_A0_or_Model82547EI_A1_or_Model82547EI_B0_Copper_or_Model82547GI_B0,
    ),
    (0x101a, Model::Model82547EI_B0_Mobile),
    (0x1010, Model::Model82546EB_A1_CopperDual),
    (0x1012, Model::Model82546EB_A1_Fiber),
    (0x1013, Model::Model82541EI_A0_or_Model82541EI_B0_Copper),
    (0x1016, Model::Model82540EP_A_Mobile),
    (0x1017, Model::Model82540EP_A_Desktop),
    (0x1018, Model::Model82541EI_B0_Mobile),
    (0x101d, Model::Model82546EB_A1_CopperQuad),
    (0x1026, Model::Model82545GM_B_Copper),
    (0x1027, Model::Model82545GM_B_Fiber),
    (0x1028, Model::Model82545GM_B_SerDes),
    (0x1076, Model::Model82541GI_B1_Copper_or_Model82541PI_C0),
    (0x1077, Model::Model82541GI_B1_Mobile),
    (0x1078, Model::Model82541PI_C0),
    (0x1079, Model::Model82546GB_B0_Copper),
    (0x107a, Model::Model82546GB_B0_Fiber),
    (0x107b, Model::Model82546GB_B0_SerDes),
    (0x1107, Model::Model82544EI_A4),
    (0x1112, Model::Model82544GC_A4),
];

impl TryFrom<u16> for Model {
    type Error = ();
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        MODELS
            .iter()
            .find(|(id, _)| *id == value)
            .map(|(_, model)| *model)
            .ok_or(())
    }
}

impl MemoryOrIo {
    /// Dump the contents of the data as hex
    ///
    /// Only the memory mapping is dumped: an io bar gives no way to ask how many ports were
    /// claimed, and reading past the end of the claim is rejected by the io port manager.
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn hex_dump(&self) {
        if let MemoryOrIo::Io(_) = self {
            crate::VGA
                .print_str_async("Skipping hex dump of an io bar\r\n")
                .await;
            return;
        }
        let mut buffer = [0u32; 32];
        for (i, b) in buffer.iter_mut().enumerate() {
            *b = self.read(i as u16);
        }
        crate::modules::video::hex_dump_generic_async(&buffer, true, false).await;
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
#[allow(non_camel_case_types)]
#[allow(unused)]
#[repr(u16)]
enum IntelPro1000Registers {
    /// Device control register
    Ctrl = 0,
    /// Device status register
    Status = 8,
    /// The eeprom read register
    Eeprom = 0x14,
    /// MDI control register
    Mdic = 0x20,
    /// Interrupt cause register
    Icr = 0xc0,
    /// Interrupt throtting register, units are 256ns, value is 16 bits out of the 32 bits for the register.
    Itr = 0xc4,
    /// Interrupt mask set/read register. Writing a one sets the corresponding bit.
    Ims = 0xd0,
    /// Interrupt mask clear register. Writing a one clears the corresponding bit.
    Imc = 0xd8,
    /// Receive control register
    Rctrl = 0x100,
    /// Transmit control register
    Tctrl = 0x400,
    /// Transmit IPG (inter packet gap) register
    Tipg = 0x410,
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
    /// Receive delay timer register
    Rdtr = 0x2820,
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
    /// Multicast table array base register
    MTA_BASE = 0x5200,
    /// Receive address low
    RAL0 = 0x5400,
    /// Receive address high
    RAH0 = 0x5404,
    /// Receive address low
    RAL1 = 0x5400 + 8,
    /// Receive address high
    RAH1 = 0x5404 + 8,
    /// Receive address low
    RAL2 = 0x5400 + 8 * 2,
    /// Receive address high
    RAH2 = 0x5404 + 8 * 2,
    /// Receive address low
    RAL3 = 0x5400 + 8 * 3,
    /// Receive address high
    RAH3 = 0x5404 + 8 * 3,
    /// Receive address low
    RAL4 = 0x5400 + 8 * 4,
    /// Receive address high
    RAH4 = 0x5404 + 8 * 4,
    /// Receive address low
    RAL5 = 0x5400 + 8 * 5,
    /// Receive address high
    RAH5 = 0x5404 + 8 * 5,
    /// Receive address low
    RAL6 = 0x5400 + 8 * 6,
    /// Receive address high
    RAH6 = 0x5404 + 8 * 6,
    /// Receive address low
    RAL7 = 0x5400 + 8 * 7,
    /// Receive address high
    RAH7 = 0x5404 + 8 * 7,
    /// Receive address low
    RAL8 = 0x5400 + 8 * 8,
    /// Receive address high
    RAH8 = 0x5404 + 8 * 8,
    /// Receive address low
    RAL9 = 0x5400 + 8 * 9,
    /// Receive address high
    RAH9 = 0x5404 + 8 * 9,
    /// Receive address low
    RAL10 = 0x5400 + 8 * 10,
    /// Receive address high
    RAH10 = 0x5404 + 8 * 10,
    /// Receive address low
    RAL11 = 0x5400 + 8 * 11,
    /// Receive address high
    RAH11 = 0x5404 + 8 * 11,
    /// Receive address low
    RAL12 = 0x5400 + 8 * 12,
    /// Receive address high
    RAH12 = 0x5404 + 8 * 12,
    /// Receive address low
    RAL13 = 0x5400 + 8 * 13,
    /// Receive address high
    RAH13 = 0x5404 + 8 * 13,
    /// Receive address low
    RAL14 = 0x5400 + 8 * 14,
    /// Receive address high
    RAH14 = 0x5404 + 8 * 14,
    /// Receive address low
    RAL15 = 0x5400 + 8 * 15,
    /// Receive address high
    RAH15 = 0x5404 + 8 * 15,
}

bitfield::bitfield! {
    /// Used to delay interrupt notification for receiving packets. Using the interrupt throttiling register (itr) is preferred.
    struct Rdtr(u32);
    impl Debug;
    impl new;
    /// The delay in 1.024us increments
    u16, counts, set_counts: 15, 0;
    /// set when flush partial descriptor block is desired
    fpd, set_fpd: 31;
}

#[derive(Clone, Default)]
/// Ethernet driver for the intel pro/1000 ethernet controller on pci
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
    /// The status of an rx descriptor
    struct DeviceControl(u32);
    impl Debug;
    impl new;
    /// full-duplex enable. This allows overriding the hardware auto-negotiation function. FRCDPLX must be set in order for duplex mode to be set.
    fd, set_fd: 0;
    /// link reset. For TBI/SerDes modes, reset the link and restart auto-negotation.
    lrst, set_lrst: 3;
    /// auto speed detection enable. SLU must also be set for this to function.
    asde, set_asde: 5;
    /// set link up. For TBI/SerDes mode this provides manual link configuration. Auto-negotation must be disabled for this to function. This must be set for internal PHY mode.
    slu, set_slu: 6;
    /// invert loss of signal. This makes the loss of signal input active low. Only for 82541xx and 82547GI/EI models.
    ilos, set_ilos: 7;
    /// the speed configuration of the link. Not used for TBI / internal Serdes modes. Ignored when auto-speed detection is enabled (asde). frcspd must be set for this to operate.
    /// 0 = 10 Mbps
    /// 1 = 100 Mbps
    /// 2 = 1000 Mbps
    /// 3 = Reserved
    u8, speed, set_speed: 9, 8;
    /// Force speed enable.
    frcspd, set_frcspd: 11;
    /// Force duplex mode enable
    frcdplx, set_frcdplx: 12;
    /// sdp0 data value. Used to read and write value of IO pin SDP0
    sdp0_data, set_spd0_data: 18;
    /// sdp1 data value. Used to read and write value of IO pin SDP1
    sdp1_data, set_spd1_data: 19;
    /// D3cold wakeup capability advertisement enable. When enabled, advertisement is based on AUX_PWR pin.
    advd3wuc, set_advd3wuc: 20;
    /// enable phy power management
    en_phy_pwr_mgmt, set_en_phy_pwr_mgmt: 21;
    /// Set SDP0 pin as an output.
    sdp0_iodir, set_sdp0_iodir: 22;
    /// Set SDP1 pin as an output.
    sdp1_iodir, set_sdp1_iodir: 23;
    /// Device reset, this is self clearing. PCI configuration registers are not cleared by this. Wait for 1us after setting before doing anything else to the device.
    rst, set_rst: 26;
    /// Receive flow control enable. Enables response to flow control packets.
    rfce, set_rfce: 27;
    /// Transmit flow control enable. Enables transmission of flow control packets (XOK and XOFF)
    tfce, set_tfce: 28;
    ///vlan mode enable. Enables vlan mode for 802.1q packets in both receive and transmit.
    vme, set_vme: 30;
    /// PHY reset. Set this, wait 3us, then clear it. For the 82546GB, hold for 10ms instead of 3us.
    phy_rst, set_phy_rst: 31;
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
///
/// The field order and types are exactly the 16 byte legacy receive descriptor. The struct is not
/// packed: the natural layout of these fields is already 16 bytes with no padding, so the fields
/// can be referenced and read volatile without creating unaligned references.
#[repr(C)]
struct RxBuffer {
    /// the physical address of the buffer
    address: u64,
    /// packet length
    length: u16,
    /// the checksum of the packet (not valid for model 82544GC/EI)
    checksum: u16,
    /// descriptor status
    status: RxBufferStatus,
    /// receive errors
    errors: RxError,
    /// extra data for 802.1q packets (not valid for model 82544GC/EI)
    special: RxSpecial,
}

const _: () = assert!(core::mem::size_of::<RxBuffer>() == 16);
const _: () = assert!(core::mem::size_of::<TxBuffer>() == 16);

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

/// A legacy TxBuffer for the device
///
/// See [RxBuffer] for why this is not packed.
#[repr(C)]
#[derive(Debug)]
struct TxBuffer {
    /// The buffer address
    address: u64,
    /// The length of the data for the descriptor
    length: u16,
    /// checksum offset, where a tcp checksum should be inserted, if enabled
    cso: u8,
    /// command field
    cmd: u8,
    /// status field
    status: u8,
    /// checksum start field, where to begin computing the checksum for tcp packets
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
            // CSS is only interpreted together with the checksum insertion command, which this
            // driver never sets. It is left at a non zero value rather than being reset per
            // packet, so that every descriptor in the ring looks the same.
            css: 1,
            special: 0,
        }
    }
}

/// The receive descriptor ring and the packet buffers the descriptors point at.
struct RxBuffers {
    /// The descriptors. One page holds `RX_DESC_COUNT` 16 byte descriptors and is page aligned,
    /// which is more aligned than the 16 byte alignment the ring requires.
    descs: DmaPage,
    /// One packet buffer per descriptor
    data: alloc::vec::Vec<DmaPage>,
}

impl RxBuffers {
    /// Construct a new receive ring with the specified number of descriptors
    fn new(quantity: usize) -> Result<Self, core::alloc::AllocError> {
        let descs = dma_page()?;
        if quantity * core::mem::size_of::<RxBuffer>() > descs.size() {
            return Err(core::alloc::AllocError);
        }
        let mut data = alloc::vec::Vec::with_capacity(quantity);
        for _i in 0..quantity {
            let page = dma_page()?;
            if page.size() < PACKET_BUFFER_SIZE {
                return Err(core::alloc::AllocError);
            }
            data.push(page);
        }
        // The device has not been told about the ring yet, so plain writes are fine here.
        for (i, page) in data.iter().enumerate() {
            unsafe {
                core::ptr::write(
                    descs.ptr::<RxBuffer>().add(i),
                    RxBuffer::new(page.phys() as u64),
                );
            }
        }
        Ok(Self { descs, data })
    }

    /// The number of descriptors in the ring
    fn count(&self) -> usize {
        self.data.len()
    }

    /// The descriptor at the specified index
    fn desc(&self, index: usize) -> *mut RxBuffer {
        // Indexing is bounded by count(), which was checked against the page size at construction.
        unsafe { self.descs.ptr::<RxBuffer>().add(index) }
    }
}

/// The transmit descriptor ring and the packet buffers the descriptors point at.
struct TxBuffers {
    /// The descriptors
    descs: DmaPage,
    /// One packet buffer per descriptor
    data: alloc::vec::Vec<DmaPage>,
}

impl TxBuffers {
    /// Construct a new transmit ring with the specified number of descriptors
    fn new(quantity: usize) -> Result<Self, core::alloc::AllocError> {
        let descs = dma_page()?;
        if quantity * core::mem::size_of::<TxBuffer>() > descs.size() {
            return Err(core::alloc::AllocError);
        }
        let mut data = alloc::vec::Vec::with_capacity(quantity);
        for _i in 0..quantity {
            let page = dma_page()?;
            if page.size() < PACKET_BUFFER_SIZE {
                return Err(core::alloc::AllocError);
            }
            data.push(page);
        }
        for (i, page) in data.iter().enumerate() {
            unsafe {
                core::ptr::write(
                    descs.ptr::<TxBuffer>().add(i),
                    TxBuffer::new(page.phys() as u64),
                );
            }
        }
        Ok(Self { descs, data })
    }

    /// The number of descriptors in the ring
    fn count(&self) -> usize {
        self.data.len()
    }

    /// The descriptor at the specified index
    fn desc(&self, index: usize) -> *mut TxBuffer {
        unsafe { self.descs.ptr::<TxBuffer>().add(index) }
    }
}

/// The internal data for the network card, used in interrupt and non-interrupt contexts
struct IntelPro1000DeviceInternal {
    /// The main way to access the device
    bar0: crate::IrqGuarded<MemoryOrIo>,
    /// The link is up
    up: AtomicBool,
    /// The rx buffers and the index for the current rx buffer
    rxbufs: crate::IrqGuarded<Option<(RxBuffers, u8)>>,
    /// Packet stream
    packet_stream_rx: Locked<Option<IrqStreamReader<super::super::RawEthernetPacket>>>,
    packet_stream_tx: IrqStreamWriter<super::super::RawEthernetPacket>,
    /// Packets dropped by the interrupt handler, because the packet spanned descriptors, was
    /// malformed, or because there was no room in the packet stream.
    rx_dropped: AtomicU32,
    /// Packets dropped by the interrupt handler because the device reported a receive error.
    rx_errors: AtomicU32,
    /// Receive overruns reported by the device.
    rx_overruns: AtomicU32,
}

impl Arc<IntelPro1000DeviceInternal> {
    /// Cache the link state from a value read from the device status register.
    ///
    /// The caller performs the register read because these guards are not reentrant: the interrupt
    /// handler holds the `bar0` guard for the whole handler, so a helper that acquired the guard
    /// itself would spin forever waiting for a lock the same context already owns.
    fn set_link_status(&self, status: u32) {
        let linkstat = (status & 2) != 0;
        self.up.store(linkstat, Ordering::Relaxed);
    }

    /// Update the status of the link from a non-interrupt context
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn update_link_status(&self) {
        let status = self
            .bar0
            .access()
            .await
            .read(IntelPro1000Registers::Status as u16);
        self.set_link_status(status);
    }
}

impl IntelPro1000DeviceInternal {
    /// Create a new Self
    fn new(bar0: crate::IrqGuarded<MemoryOrIo>, up: AtomicBool, common: &IrqGuardedInner) -> Self {
        let p = new_irq_stream(common, 10, 5);
        Self {
            bar0,
            up,
            rxbufs: IrqGuarded::new(None, common),
            packet_stream_rx: Locked::new(Some(p.0)),
            packet_stream_tx: p.1,
            rx_dropped: AtomicU32::new(0),
            rx_errors: AtomicU32::new(0),
            rx_overruns: AtomicU32::new(0),
        }
    }
}

/// The actual intel pro/1000 device
pub struct IntelPro1000Device {
    /// The internal structure used in interrupt handler and regular code
    internal: Arc<IntelPro1000DeviceInternal>,
    /// The base address registers
    _bars: [Option<BarSpace>; 6],
    /// the io space allocated for the device. It is only used for an attached flash on some
    /// boards, so it is only claimed so that nothing else can use it.
    _io: Option<crate::IoPortArray<'static>>,
    /// Is the eeprom present?
    eeprom_present: Option<bool>,
    /// The tx buffers
    txbufs: Option<TxBuffers>,
    /// The current tx buffer
    txbufindex: usize,
    /// The specific model of the device
    model: Model,
    /// The mac address
    mac_address: MacAddress,
    /// The address the phy responds to on the mdi bus, once discovered
    phy_address: Option<u8>,
    /// The number of transmissions that did not complete
    tx_errors: AtomicU32,
    /// The receiver for packets to send out
    recvr: IrqStreamReader<super::super::RawEthernetPacket>,
    /// The sender half for packets to send out
    sendr: IrqStreamWriter<super::super::RawEthernetPacket>,
}

bitflags::bitflags! {
    /// The receive control register flags.
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
        /// Loopback enabled. Only allowed for full-duplex operations. Not supported by 82540EP/EM, 82541XX, and 82547GI/EI models.
        const LOOPBACK = 3<<6;
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

/// Fields of the receive control register that are not single bits. They live here rather than in
/// [RctrlFlags] because bitflags entries are expected to be single bits, and several of the
/// encodings below share the value zero with "no flags". The other encodings of each field are
/// 1/4 and 1/8 of RDLEN for the minimum threshold (1 << 8 and 2 << 8) and multicast offsets using
/// bits [46:35], [45:34] and [43:32] (1 << 12, 2 << 12 and 3 << 12).
impl RctrlFlags {
    /// Set the receive descriptor minimum threshold size to 1/2 of RDLEN
    const RDMTS_HALF: u32 = 0;
    /// Multicast offset use bits [47:36]
    const MO_36: u32 = 0;
}

bitflags::bitflags! {
    /// The transmit control register flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct TctrlFlags: u32 {
        /// Transmit enable. After disabling, the transmitter should be reset before enabling.
        const EN = 1<<1;
        /// pad short packets enable. Makes short packets 64 bytes long by padding with data bytes, otherwise the minimum packet length is 32 bytes. Not the same as minimum collision distance.
        const PSP = 1<<3;
        /// software XOFF transmission. schedules the transmission of an XOFF (puase) frame using the current value of the PAUSe timer (FCTTV.TTV)
        const SWXOFF = 1<<22;
        /// retransmit on late colision. Enables retransmit when there is a late collision event. Collision window is speed dependent. (64 bytes for 10/100 Mbps, 512 bytes for 1000 Mbps). Only for half-duplex mode.
        const RTLC = 1<<24;
        /// No re-transmit on underrun (8244GC/EI only)
        const NRTU = 1<<25;
    }
}

/// The multi bit fields of the transmit control register. Kept out of [TctrlFlags] for the same
/// reason as the [RctrlFlags] equivalents. The fields are 8 bits wide for the collision threshold
/// at bit 4 and 10 bits wide for the collision distance at bit 12.
impl TctrlFlags {
    /// collision threshold shift.
    const CT_SHIFT: u32 = 4;
    /// collision distance shift.
    const COLD_SHIFT: u32 = 12;
    /// The collision threshold programmed into the register.
    const COLLISION_THRESHOLD: u32 = 15 << Self::CT_SHIFT;
    /// The collision distance programmed into the register.
    const COLLISION_DISTANCE: u32 = 64 << Self::COLD_SHIFT;
}

bitflags::bitflags! {
    /// The command field of a legacy transmit descriptor.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct TxCmdFlags: u32 {
        /// End of packet. This is the last descriptor of the packet.
        const EOP = 1<<0;
        /// Insert the frame checksum.
        const IFCS = 1<<1;
        /// Insert the tcp/udp checksum, using CSS and CSO.
        const IC = 1<<2;
        /// Report status. The device writes back the descriptor status field when done.
        const RS = 1<<3;
        /// Report packet sent. The device writes back the status again when the packet is on the wire.
        const RPS = 1<<4;
        /// Descriptor extension. When set the descriptor is an advanced descriptor; this driver only uses legacy descriptors.
        const DEXT = 1<<5;
        /// 802.1q vlan packet enable.
        const VLE = 1<<6;
    }
}

bitflags::bitflags! {
    /// The interrupt mask bits, used with the interrupt mask set, clear, and cause registers.
    ///
    /// The mask set register sets the bits written to it and the mask clear register clears the
    /// bits written to it; the cause register is cleared by reading it.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct InterruptMask: u32 {
        /// Transmit descriptor write back
        const TXDW = 1<<0;
        /// Transmit queue empty
        const TXQE = 1<<1;
        /// Link status change
        const LSC = 1<<2;
        /// Receive sequence error
        const RXSEQ = 1<<3;
        /// Receive descriptor minimum threshold reached
        const RXDMT = 1<<4;
        /// Receiver overrun
        const RXO = 1<<6;
        /// Receiver timer interrupt
        const RXT = 1<<7;
        /// MDIO access complete
        const MDAC = 1<<9;
        /// Receiving ordered sets
        const RXCFG = 1<<10;
        /// Phy interrupt
        const PHYINT = 1<<12;
        /// Transmit descriptor low threshold
        const TXD_LOW = 1<<15;
        /// Small receive packet
        const SRPD = 1<<16;
    }
}

bitflags::bitflags! {
    /// The eeprom/flash control register flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct EecFlags: u32 {
        /// Chip select for the eeprom. Must be cleared again when the access is finished.
        const EECS = 1<<0;
        /// Data in to the eeprom
        const EEDI = 1<<1;
        /// Data out of the eeprom
        const EEDO = 1<<2;
        /// The eeprom clock
        const CLK = 1<<3;
        /// The current access is complete. Set by the device, cleared by the next access.
        const DONE = 1<<4;
    }
}

/// The shift of the eeprom address field of the eeprom control register.
const EEC_ADDR_SHIFT: u32 = 8;
/// The shift of the eeprom read data field of the eeprom control register.
const EEC_DATA_SHIFT: u32 = 16;
/// The mdi read opcode
const MDIC_READ: u8 = 2;
/// The phy basic status register
const PHY_REG_STATUS: u8 = 1;

bitfield::bitfield! {
    /// The status of an rx descriptor
    struct ReceiveAddress(u64);
    impl Debug;
    impl new;
    /// The mac address
    u64, from into MacAddress, mac, set_mac: 47, 0;
    /// How the address is to be used in address filtering
    /// 0 = Use destination address (normal mode)
    /// 1 = Source address
    /// 2 = Reserved
    /// 3 = Reserved
    u8, address_select, set_address_select: 49, 48;
    /// Set when the address is compared against incoming packets
    address_valid, set_address_valid: 63;
    /// The low 32 bits of the struct
    u32, low, _ : 31, 0;
    /// The high 32 bits of the struct
    u32, high, _ : 63, 32;
}

bitfield::bitfield! {
    /// The MDIC register
    struct MdicRegister(u32);
    impl Debug;
    impl new;
    /// The data for a write command
    u16, data, set_data: 15, 0;
    /// The phy register address
    u8, regadd, set_regadd: 20, 16;
    /// The phy address
    u8, phyadd, set_phyadd: 25, 21;
    /// The opcode. 1 = write, 2 = read
    u8, opcode, set_opcode: 27, 26;
    /// The ready flag
    ready, _ : 28;
    /// Interrupt enable
    i, set_i: 29;
    /// Error flag
    error, _: 30;
}

bitfield::bitfield! {
    /// The interrupt cause register
    struct InterruptCauseRegister(u32);
    impl Debug;
    impl new;
    /// Transmit descriptor write back
    txdw, _: 0;
    /// Transmit queue empty
    txqe, _: 1;
    /// link status change
    lsc, _: 2;
    /// receive sequence error
    rxseq, _: 3;
    /// receive descriptor minimum threshold reached
    rxdmt, _: 4;
    /// receiver overrun
    rxo, _: 6;
    /// receiver timer interrupt
    rxt, _: 7;
    /// mdio access complete
    mdac, _: 9;
    /// receiving ordered sets
    rxcfg, _: 10;
    /// phy interrupt
    phyint, _: 12;
    /// general interrupt on SPD6[2]
    gpi_sdp6, _: 13;
    /// general interrupt in SDP7[3]
    gpi_sdp7, _: 14;
    /// general purpose interrupt (bit 1 not used)
    u8, gpi, _: 14, 11;
    /// transmit descriptor low threshold
    txd_low, _: 15;
    /// small receive packet
    srpd, _: 16;
}

/// Why a transmission failed.
#[derive(Debug)]
enum SendError {
    /// The packet needs more than one transmit descriptor, which is not implemented yet.
    TooLong(usize),
    /// The device has not been initialized.
    NotInitialized,
    /// The link never came up.
    NoLink,
    /// The device never wrote back the descriptor.
    Timeout,
}

impl NetworkAdapterTrait for IntelPro1000Device {
    fn get_receiver(&self) -> Option<IrqStreamReader<super::super::RawEthernetPacket>> {
        let mut a = self.internal.packet_stream_rx.sync_lock();
        a.take()
    }

    fn get_sender(&self) -> IrqStreamWriter<crate::modules::network::RawEthernetPacket> {
        self.sendr.clone()
    }

    async fn send_pending_packets(mut self) {
        loop {
            if let Some(p) = self.recvr.next().await {
                net_print!("Sending a packet of {} bytes\r\n", p.length);
                if let Err(e) = self.send_packet(&p.data[0..p.length]).await {
                    crate::VGA
                        .print_str_async(&format!("Failed to send packet {:?}\r\n", e))
                        .await;
                }
            }
        }
    }

    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn get_mac_address(&mut self) -> MacAddress {
        if self.detect_eeprom().await {
            let words = (
                self.read_from_eeprom(0).await,
                self.read_from_eeprom(1).await,
                self.read_from_eeprom(2).await,
            );
            if let (Some(w0), Some(w1), Some(w2)) = words {
                let w0 = w0.to_le_bytes();
                let w1 = w1.to_le_bytes();
                let w2 = w2.to_le_bytes();
                return MacAddress {
                    address: [w0[0], w0[1], w1[0], w1[1], w2[0], w2[1]],
                };
            }
            crate::VGA
                .print_str_async("Eeprom read failed, using the receive address register\r\n")
                .await;
        } else {
            crate::VGA
                .print_str_async("No eeprom, using the receive address register for the mac\r\n")
                .await;
        }
        // Without an eeprom the only source of a mac address is whatever the firmware or a
        // previous driver left in receive address register 0.
        let ra = self.get_receive_mac_address(0).await;
        let mac = ra.mac();
        if u64::from(mac) == 0 {
            crate::VGA
                .print_str_async("Warning: the mac address is all zeroes\r\n")
                .await;
        }
        mac
    }
}

impl IntelPro1000Device {
    /// Reset the device and program the registers that must be programmed after every reset.
    ///
    /// This must run before any configuration: the reset returns the device to its power on state.
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn reset(&mut self) {
        {
            let mut bar0 = self.internal.bar0.access().await;
            let mut ctrl = DeviceControl(bar0.read(IntelPro1000Registers::Ctrl as u16));
            ctrl.set_rst(true);
            bar0.write(IntelPro1000Registers::Ctrl as u16, ctrl.0);
        }
        // The datasheet requires waiting after the reset request before touching the device again.
        yield_delay(RESET_SETTLE_YIELDS).await;
        let ctrl = DeviceControl(
            self.internal
                .bar0
                .access()
                .await
                .read(IntelPro1000Registers::Ctrl as u16),
        );
        if ctrl.rst() {
            crate::VGA
                .print_str_async("Warning: device reset did not complete\r\n")
                .await;
        }

        // Phy reset. The request is held, then released.
        {
            let mut bar0 = self.internal.bar0.access().await;
            let mut ctrl = DeviceControl(bar0.read(IntelPro1000Registers::Ctrl as u16));
            ctrl.set_phy_rst(true);
            bar0.write(IntelPro1000Registers::Ctrl as u16, ctrl.0);
        }
        yield_delay(PHY_RESET_HOLD_YIELDS).await;
        {
            let mut bar0 = self.internal.bar0.access().await;
            let mut ctrl = DeviceControl(bar0.read(IntelPro1000Registers::Ctrl as u16));
            ctrl.set_phy_rst(false);
            bar0.write(IntelPro1000Registers::Ctrl as u16, ctrl.0);
        }

        // The inter packet gap register is zeroed by the reset and must be written before
        // anything is transmitted. Fiber and SerDes models use a different value.
        let mut bar0 = self.internal.bar0.access().await;
        bar0.write(IntelPro1000Registers::Tipg as u16, TIPG_COPPER);
    }

    /// Wait for the link to come up.
    ///
    /// The link status change interrupt normally keeps the cached state current; polling as well
    /// means a missed change cannot wedge the sender forever.
    async fn wait_for_link(&self) -> Result<(), SendError> {
        for _ in 0..LINK_POLLS {
            if self.internal.up.load(Ordering::Relaxed) {
                return Ok(());
            }
            self.internal.update_link_status().await;
            yield_delay(LINK_POLL_YIELDS).await;
        }
        if self.internal.up.load(Ordering::Relaxed) {
            Ok(())
        } else {
            Err(SendError::NoLink)
        }
    }

    /// Transmit one packet.
    ///
    /// The packet is copied into the next transmit buffer, the source mac address is filled in,
    /// and the descriptor is handed to the device. Only one transmission is in flight at a time:
    /// the call waits for the device to write back the descriptor.
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn send_packet(&mut self, packet: &[u8]) -> Result<(), ()> {
        match self.try_send_packet(packet).await {
            Ok(()) => Ok(()),
            Err(e) => {
                self.tx_errors.fetch_add(1, Ordering::Relaxed);
                net_print!("Transmit failed {:?}\r\n", e);
                Err(())
            }
        }
    }

    /// The implementation of [Self::send_packet].
    async fn try_send_packet(&mut self, packet: &[u8]) -> Result<(), SendError> {
        if packet.len() > PACKET_BUFFER_SIZE {
            doors_macros::todo_item!("Transmit packets that need more than one descriptor");
            return Err(SendError::TooLong(packet.len()));
        }
        self.wait_for_link().await?;
        let len = packet.len();
        let source = u64::from(self.mac_address).to_le_bytes();
        let txbufs = self.txbufs.as_mut().ok_or(SendError::NotInitialized)?;
        let index = self.txbufindex;
        let next = (index + 1) % txbufs.count();
        {
            let buffer = txbufs.data[index].bytes_mut();
            buffer[..len].copy_from_slice(packet);
            // The upper layers leave the source address empty.
            buffer[6..12].copy_from_slice(&source[..6]);
        }
        let desc = txbufs.desc(index);
        // SAFETY: index is within the ring. The device is not given the descriptor until the tail
        // pointer is written below, and the previous transmission of this slot completed before
        // this function returned.
        unsafe {
            core::ptr::write_volatile(&mut (*desc).length, len as u16);
            core::ptr::write_volatile(&mut (*desc).cso, 0);
            core::ptr::write_volatile(
                &mut (*desc).cmd,
                (TxCmdFlags::EOP | TxCmdFlags::IFCS | TxCmdFlags::RS).bits() as u8,
            );
            core::ptr::write_volatile(&mut (*desc).status, 0);
            core::ptr::write_volatile(&mut (*desc).special, 0);
        }
        // The buffer and descriptor must reach memory before the device is told about them.
        core::sync::atomic::compiler_fence(Ordering::SeqCst);
        self.internal
            .bar0
            .access()
            .await
            .write(IntelPro1000Registers::TxDescTail as u16, next as u32);
        // Always move to the next slot. The tail pointer committed this descriptor to the device,
        // so reusing the slot while the write back is outstanding could race with the device
        // reading the buffer.
        self.txbufindex = next;
        let mut tries = 0;
        loop {
            let status = unsafe { core::ptr::read_volatile(&(*desc).status) };
            if (status & DESC_STATUS_DD) != 0 {
                return Ok(());
            }
            tries += 1;
            if tries >= TX_COMPLETE_TRIES {
                return Err(SendError::Timeout);
            }
            crate::executor::AsyncTask::yield_now().await;
        }
    }

    /// Detect the presence of an eeprom and store the result
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn detect_eeprom(&mut self) -> bool {
        if self.eeprom_present.is_none() {
            let mut present = false;
            {
                let mut bar0 = self.internal.bar0.access().await;
                bar0.write(IntelPro1000Registers::Eeprom as u16, EecFlags::EECS.bits());
                for _i in 0..EEPROM_WAIT_TRIES {
                    let val = bar0.read(IntelPro1000Registers::Eeprom as u16);
                    if (val & EecFlags::DONE.bits()) != 0 {
                        present = true;
                        break;
                    }
                    crate::executor::AsyncTask::yield_now().await;
                }
                // Deselect the eeprom again, otherwise it keeps driving the mdi bus.
                bar0.write(IntelPro1000Registers::Eeprom as u16, 0);
            }
            self.eeprom_present = Some(present);
        }
        self.eeprom_present.unwrap()
    }

    /// Dump the contents of key registers
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn dump_registers(&self) {
        let regs = &[
            IntelPro1000Registers::Ctrl,
            IntelPro1000Registers::Status,
            IntelPro1000Registers::Icr,
            IntelPro1000Registers::Ims,
            IntelPro1000Registers::Itr,
            IntelPro1000Registers::Rctrl,
            IntelPro1000Registers::RxDescLow,
            IntelPro1000Registers::RxDescHigh,
            IntelPro1000Registers::RxDescLen,
            IntelPro1000Registers::RxDescHead,
            IntelPro1000Registers::RxDescTail,
            IntelPro1000Registers::Tctrl,
            IntelPro1000Registers::Tipg,
            IntelPro1000Registers::TxDescLow,
            IntelPro1000Registers::TxDescHigh,
            IntelPro1000Registers::TxDescHead,
            IntelPro1000Registers::TxDescTail,
        ];
        let bar0 = self.internal.bar0.access().await;
        for r in regs {
            net_print!("{:?} register is {:x}\r\n", r, bar0.read(*r as u16));
        }
        net_print!(
            "rx dropped {} rx errors {} rx overruns {} tx errors {}\r\n",
            self.internal.rx_dropped.load(Ordering::Relaxed),
            self.internal.rx_errors.load(Ordering::Relaxed),
            self.internal.rx_overruns.load(Ordering::Relaxed),
            self.tx_errors.load(Ordering::Relaxed)
        );
    }

    /// Get the two registers for the receive mac address at the specified index
    fn receive_mac_address_registers(index: u8) -> (IntelPro1000Registers, IntelPro1000Registers) {
        match index {
            0 => (IntelPro1000Registers::RAL0, IntelPro1000Registers::RAH0),
            1 => (IntelPro1000Registers::RAL1, IntelPro1000Registers::RAH1),
            2 => (IntelPro1000Registers::RAL2, IntelPro1000Registers::RAH2),
            3 => (IntelPro1000Registers::RAL3, IntelPro1000Registers::RAH3),
            4 => (IntelPro1000Registers::RAL4, IntelPro1000Registers::RAH4),
            5 => (IntelPro1000Registers::RAL5, IntelPro1000Registers::RAH5),
            6 => (IntelPro1000Registers::RAL6, IntelPro1000Registers::RAH6),
            7 => (IntelPro1000Registers::RAL7, IntelPro1000Registers::RAH7),
            8 => (IntelPro1000Registers::RAL8, IntelPro1000Registers::RAH8),
            9 => (IntelPro1000Registers::RAL9, IntelPro1000Registers::RAH9),
            10 => (IntelPro1000Registers::RAL10, IntelPro1000Registers::RAH10),
            11 => (IntelPro1000Registers::RAL11, IntelPro1000Registers::RAH11),
            12 => (IntelPro1000Registers::RAL12, IntelPro1000Registers::RAH12),
            13 => (IntelPro1000Registers::RAL13, IntelPro1000Registers::RAH13),
            14 => (IntelPro1000Registers::RAL14, IntelPro1000Registers::RAH14),
            15 => (IntelPro1000Registers::RAL15, IntelPro1000Registers::RAH15),
            _ => panic!("Invalid index {}", index),
        }
    }

    /// Set the receive address of the specified index
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn set_receive_mac_address(&mut self, index: u8, ra: &ReceiveAddress) {
        let (low, high) = Self::receive_mac_address_registers(index);
        let mut bar0 = self.internal.bar0.access().await;
        bar0.write(low as u16, ra.low());
        bar0.write(high as u16, ra.high());
    }

    /// Clear the receive address at the specified index
    async fn clear_receive_mac_address(&mut self, index: u8) {
        let (low, high) = Self::receive_mac_address_registers(index);
        let mut bar0 = self.internal.bar0.access().await;
        // Clear the high word first: it holds the valid bit, so the address stops being used
        // before the address itself disappears.
        bar0.write(high as u16, 0u32);
        bar0.write(low as u16, 0u32);
    }

    /// Retrieve the existing receive address at the specified index from the device
    async fn get_receive_mac_address(&mut self, index: u8) -> ReceiveAddress {
        let (low, high) = Self::receive_mac_address_registers(index);
        let bar0 = self.internal.bar0.access().await;
        let ral = bar0.read(low as u16);
        let rah = bar0.read(high as u16);
        let combined: u64 = ((rah as u64) << 32) | (ral as u64);
        ReceiveAddress(combined)
    }

    /// Clear the multicast table array
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn clear_multicast_table_array(&mut self) {
        let base = IntelPro1000Registers::MTA_BASE as u16;
        let end = base + 0x200;
        let mut bar0 = self.internal.bar0.access().await;
        for r in (base..end).step_by(4) {
            bar0.write(r, 0u32);
        }
    }

    /// Read a u16 from the specified phy
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn read_from_phy(&mut self, phy: u8, index: u8) -> Option<u16> {
        let mut bar0 = self.internal.bar0.access().await;
        bar0.write(
            IntelPro1000Registers::Mdic as u16,
            MdicRegister::new(0, index, phy, MDIC_READ, false).0,
        );

        let mut tries = 0;
        loop {
            let mdic = MdicRegister(bar0.read(IntelPro1000Registers::Mdic as u16));
            if mdic.ready() {
                break;
            }
            tries += 1;
            if tries >= MDIC_WAIT_TRIES {
                // The mdi bus never answered.
                return None;
            }
            crate::executor::AsyncTask::yield_now().await;
        }
        let mdic = MdicRegister(bar0.read(IntelPro1000Registers::Mdic as u16));
        if mdic.error() {
            None
        } else {
            Some(mdic.data())
        }
    }

    /// Work out which mdi bus address the phy responds to.
    ///
    /// The address is strapped by board hardware, so it cannot be assumed. The addresses boards
    /// actually use are tried first so that the common case costs two transactions.
    async fn detect_phy_address(&mut self) -> Option<u8> {
        for phy in (0..2u8).chain(2..32) {
            if let Some(status) = self.read_from_phy(phy, PHY_REG_STATUS).await {
                if status != 0 && status != 0xffff {
                    self.phy_address = Some(phy);
                    return Some(phy);
                }
            }
        }
        None
    }

    /// Perform a general configuration of the device, to be performed after power-up or after reset.
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn general_config(&mut self) {
        if matches!(
            self.model,
            Model::Model82541EI_A0_or_Model82541EI_B0_Copper
                | Model::Model82541EI_B0_Mobile
                | Model::Model82541GI_B1_Copper_or_Model82541PI_C0
                | Model::Model82541GI_B1_Mobile
                | Model::Model82541PI_C0
                | Model::Model82547EI_A0_or_Model82547EI_A1_or_Model82547EI_B0_Copper_or_Model82547GI_B0
                | Model::Model82547EI_B0_Mobile
        ) {
            // The datasheet asks for the led behavior to be configured and the statistics counters
            // to be cleared by reading them on these models. Neither is done, which leaves the leds
            // in their power on state and the counters counting from whatever the boot loader left
            // behind. Neither is fatal, and doing it here with todo!() used to stop the device from
            // initializing at all.
            doors_macros::todo_item!("Configure LED behavior and clear statistics counters");
        }
        let mut bar0 = self.internal.bar0.access().await;
        let mut ctrl = DeviceControl(bar0.read(IntelPro1000Registers::Ctrl as u16));
        if !ctrl.slu() {
            // The internal phy needs the link to be configured by software.
            ctrl.set_slu(true);
            bar0.write(IntelPro1000Registers::Ctrl as u16, ctrl.0);
        }
    }

    /// Initialize the rx buffers for the device
    ///
    /// The ring is allocated before the device is touched, so an allocation failure leaves the
    /// receiver unconfigured. No two guards are held at once here: the interrupt handler takes
    /// `bar0` before `rxbufs`, and doing register programming before taking the ring lock keeps
    /// the asynchronous side from ever inverting that order.
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn init_rx(&mut self, mac: &MacAddress) -> Result<(), core::alloc::AllocError> {
        let rxbuf = RxBuffers::new(RX_DESC_COUNT)?;
        let rxaddr = rxbuf.descs.phys();

        let ra = ReceiveAddress::new(*mac, 0, true);
        net_print!("Setting receive address to: {:02x?}\r\n", ra);
        self.set_receive_mac_address(0, &ra).await;
        let ra2 = self.get_receive_mac_address(0).await;
        net_print!("Reading receive address as: {:02x?}\r\n", ra2);
        // Invalidate the addresses left over from the boot loader or a previous driver.
        for index in 1..16 {
            self.clear_receive_mac_address(index).await;
        }
        self.clear_multicast_table_array().await;

        let mut bar0 = self.internal.bar0.access().await;
        bar0.write(
            IntelPro1000Registers::RxDescLow as u16,
            (rxaddr & 0xFFFFFFFF) as u32,
        );
        bar0.write(
            IntelPro1000Registers::RxDescHigh as u16,
            (rxaddr >> 32) as u32,
        );
        bar0.write(
            IntelPro1000Registers::RxDescLen as u16,
            core::mem::size_of::<RxBuffer>() as u32 * rxbuf.count() as u32,
        );
        bar0.write(IntelPro1000Registers::RxDescHead as u16, 0);
        // See the ring conventions at the top of the file: the tail names the descriptor after the
        // last one hardware may write, so the last descriptor of the ring is the spare that keeps
        // a full ring distinguishable from an empty one.
        bar0.write(
            IntelPro1000Registers::RxDescTail as u16,
            rxbuf.count() as u32 - 1,
        );
        // SECRC strips the frame checksum so that the length reported for a packet is the length
        // of the ethernet frame. MPE accepts all multicast packets; a multicast table hash filter
        // would be the more selective replacement.
        bar0.write(
            IntelPro1000Registers::Rctrl as u16,
            (RctrlFlags::EN
                | RctrlFlags::BAM
                | RctrlFlags::MPE
                | RctrlFlags::SECRC
                | RCTL_BUFFER_SIZE)
                .bits()
                | RctrlFlags::RDMTS_HALF
                | RctrlFlags::MO_36,
        );
        drop(bar0);

        let mut rxbufs = self.internal.rxbufs.access().await;
        rxbufs.replace((rxbuf, 0));
        Ok(())
    }

    /// Initialize the tx buffers for the device
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn init_tx(&mut self) -> Result<(), core::alloc::AllocError> {
        if self.txbufs.is_none() {
            let txbuf = TxBuffers::new(TX_DESC_COUNT)?;
            let txaddr = txbuf.descs.phys();
            let mut bar0 = self.internal.bar0.access().await;
            bar0.write(
                IntelPro1000Registers::TxDescLow as u16,
                (txaddr & 0xFFFFFFFF) as u32,
            );
            bar0.write(
                IntelPro1000Registers::TxDescHigh as u16,
                (txaddr >> 32) as u32,
            );
            let desclen = core::mem::size_of::<TxBuffer>() as u32 * txbuf.count() as u32;
            bar0.write(IntelPro1000Registers::TxDescLen as u16, desclen);
            bar0.write(IntelPro1000Registers::TxDescHead as u16, 0);
            // Both pointers at zero means nothing has been given to the device yet.
            bar0.write(IntelPro1000Registers::TxDescTail as u16, 0);

            bar0.write(
                IntelPro1000Registers::Tctrl as u16,
                (TctrlFlags::EN | TctrlFlags::PSP | TctrlFlags::RTLC).bits()
                    | TctrlFlags::COLLISION_THRESHOLD
                    | TctrlFlags::COLLISION_DISTANCE,
            );
            self.txbufindex = 0;
            self.txbufs = Some(txbuf);
        }
        Ok(())
    }

    /// The interrupt handler for the network card
    ///
    /// Nothing in here may block, print, or panic: it runs with the device interrupt masked and
    /// holds the device locks. Packets that cannot be used are dropped and counted instead.
    fn handle_interrupt(
        this: &Arc<IntelPro1000DeviceInternal>,
        packet: &mut Box<super::super::RawEthernetPacket>,
    ) {
        let mut bar0 = this.bar0.interrupt_access();
        // The passes are bounded so that a device raising causes faster than they can be serviced
        // cannot hold the interrupt line forever.
        for _pass in 0..MAX_INTERRUPT_PASSES {
            // Reading the cause register acknowledges everything it reports.
            let reason = InterruptCauseRegister(bar0.read(IntelPro1000Registers::Icr as u16));
            if reason.0 == 0 {
                break;
            }
            if reason.lsc() {
                // Read with the guard already held; see [IntelPro1000DeviceInternal::set_link_status].
                let status = bar0.read(IntelPro1000Registers::Status as u16);
                this.set_link_status(status);
            }
            if reason.rxo() {
                this.rx_overruns.fetch_add(1, Ordering::Relaxed);
            }
            if !(reason.rxdmt() || reason.rxt() || reason.rxseq() || reason.rxo()) {
                continue;
            }
            let mut rxbufs = this.rxbufs.interrupt_access();
            let Some((ring, index)) = rxbufs.as_mut() else {
                // The receiver has not been set up yet.
                break;
            };
            // Bound the work by the size of the ring.
            for _ in 0..ring.count() {
                // The head pointer is the descriptor the device will write next, so every
                // descriptor between the consumer index and the head has been fetched.
                let head = bar0.read(IntelPro1000Registers::RxDescHead as u16) as usize;
                if *index as usize == head {
                    break;
                }
                let desc = ring.desc(*index as usize);
                let status = unsafe { core::ptr::read_volatile(&(*desc).status) };
                if !status.dd() {
                    // Fetched but not written back yet. The next interrupt picks this up.
                    break;
                }
                let length = unsafe { core::ptr::read_volatile(&(*desc).length) } as usize;
                let errors = unsafe { core::ptr::read_volatile(&(*desc).errors) };
                // Make sure the buffer read below cannot be moved above the status check.
                core::sync::atomic::compiler_fence(Ordering::Acquire);

                if !status.eop() {
                    // The packet spans more than one descriptor. Not supported yet; drop the
                    // descriptor and move on rather than panicking in interrupt context.
                    doors_macros::todo_item!("Process a packet covering more than one descriptor");
                    this.rx_dropped.fetch_add(1, Ordering::Relaxed);
                } else if errors.0 != 0 {
                    this.rx_errors.fetch_add(1, Ordering::Relaxed);
                } else if length == 0 || length > PACKET_BUFFER_SIZE {
                    // The length comes from the device, so it is checked against the size of the
                    // buffer it indexes into rather than against the size of the packet struct.
                    this.rx_dropped.fetch_add(1, Ordering::Relaxed);
                } else {
                    // The device wrote the payload before it wrote back the descriptor, so the
                    // buffer can be read normally now.
                    let payload = &ring.data[*index as usize].bytes()[..length];
                    packet.copy(payload);
                    if this.packet_stream_tx.push_interrupt(**packet).is_err() {
                        this.rx_dropped.fetch_add(1, Ordering::Relaxed);
                    }
                }

                // Hand the descriptor back to the device. Clearing the status byte is what marks
                // the buffer as free again; leaving it set makes the device treat the descriptor
                // as still owned by the driver, and the driver would re-deliver the stale packet.
                unsafe {
                    core::ptr::write_volatile(&mut (*desc).status, RxBufferStatus(0));
                    core::ptr::write_volatile(&mut (*desc).length, 0);
                }
                core::sync::atomic::compiler_fence(Ordering::Release);
                *index = ((*index as usize + 1) % ring.count()) as u8;
                bar0.write(IntelPro1000Registers::RxDescTail as u16, *index as u32);
            }
        }
    }

    /// Enable interrupts for the network card
    ///
    /// The causes serviced by [Self::handle_interrupt] are enabled, and nothing else: transmit
    /// completion is polled, so the transmit causes would only add interrupts. Link status change
    /// is left enabled so that link flaps after initialization are noticed.
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn enable_interrupts(&mut self, sys: &System, irqnum: u8) {
        use crate::kernel::SystemTrait;
        crate::VGA
            .print_str_async(&format!("Enabling interrupts on IRQ {}\r\n", irqnum))
            .await;
        let enable = InterruptMask::LSC
            | InterruptMask::RXSEQ
            | InterruptMask::RXDMT
            | InterruptMask::RXO
            | InterruptMask::RXT;
        {
            let mut bar0 = self.internal.bar0.access().await;
            // Disable the receive delay timer: report a received packet as soon as it is written
            // back rather than after a delay.
            bar0.write(IntelPro1000Registers::Rdtr as u16, Rdtr::new(0, false).0);
            bar0.write(IntelPro1000Registers::Itr as u16, ITR_1MS);
            // The set register can only set bits, so clear everything first and then set only the
            // causes that are actually serviced.
            bar0.write(IntelPro1000Registers::Imc as u16, !0u32);
            bar0.write(IntelPro1000Registers::Ims as u16, enable.bits());
            // Check that the mask took. Only the requested bits are compared, since unsupported
            // bits read back as zero. This is bounded, and giving up disables the device's
            // interrupts rather than spinning with the device locked forever.
            let mut tries = 0;
            loop {
                let read = bar0.read(IntelPro1000Registers::Ims as u16);
                if read & enable.bits() == enable.bits() {
                    break;
                }
                tries += 1;
                if tries > IMS_VERIFY_TRIES {
                    bar0.write(IntelPro1000Registers::Imc as u16, !0u32);
                    crate::VGA
                        .print_str_async("Interrupt mask did not take, interrupts left off\r\n")
                        .await;
                    return;
                }
                bar0.write(IntelPro1000Registers::Ims as u16, enable.bits());
                crate::executor::AsyncTask::yield_now().await;
            }
            // Reading the cause register clears anything raised during setup.
            let _ = bar0.read(IntelPro1000Registers::Icr as u16);
        }

        let c = self.internal.clone();
        let mut packet_buffer = super::super::RawEthernetPacket::new_box();
        sys.register_irq_handler(irqnum, move || {
            IntelPro1000Device::handle_interrupt(&c, &mut packet_buffer)
        });
        sys.enable_irq(irqnum);
    }

    /// Read a word from the eeprom at the specified address
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn read_from_eeprom(&mut self, addr: u8) -> Option<u16> {
        if !self.detect_eeprom().await {
            return None;
        }
        let mut bar0 = self.internal.bar0.access().await;
        bar0.write(
            IntelPro1000Registers::Eeprom as u16,
            EecFlags::EECS.bits() | ((addr as u32) << EEC_ADDR_SHIFT),
        );
        let mut value = None;
        for _i in 0..EEPROM_WAIT_TRIES {
            let a = bar0.read(IntelPro1000Registers::Eeprom as u16);
            if (a & EecFlags::DONE.bits()) != 0 {
                value = Some((a >> EEC_DATA_SHIFT) as u16);
                break;
            }
            crate::executor::AsyncTask::yield_now().await;
        }
        // Deselect the eeprom again.
        bar0.write(IntelPro1000Registers::Eeprom as u16, 0);
        value
    }
}

impl IntelPro1000 {
    /// Create a new self, in const form
    pub const fn new() -> Self {
        Self {}
    }
}

impl PciFunctionDriverTrait for IntelPro1000 {
    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn register(&self, m: &mut BTreeMap<u32, PciFunctionDriver>) {
        crate::VGA
            .print_str_async("Register intel pro/1000 pci driver\r\n")
            .await;
        for (dev, _) in MODELS {
            let vendor_combo = ((*dev as u32) << 16) | 0x8086;
            m.entry(vendor_combo).or_insert_with(|| self.clone().into());
        }
    }

    #[cfg_attr(feature = "backtrace", doors_macros::framed)]
    async fn parse_bars(
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
                    crate::VGA
                        .print_str_async(&format!("PCI PARSE BAR {}\r\n", bar.get_index()))
                        .await;
                    bar.print().await;
                    let d = bar.get_memory(cs, bus, dev, f, config);
                    if let Some(d) = d {
                        crate::VGA
                            .print_str_async(&format!("Got memory at {:x}\r\n", d.virt()))
                            .await;
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
        // Some boards expose an io bar for an attached flash. It is not used, only claimed.
        let io = bars.iter_mut().find_map(|a| {
            if let Some(a) = a {
                a.get_io(cs, bus, dev, f, config)
            } else {
                None
            }
        });
        let configspace = f.get_all_configuration(cs, bus, dev);
        configspace.dump("\t").await;
        let Some(bar0) = bar0 else {
            crate::VGA
                .print_str_async("Intel pro/1000 has no memory bar, not binding the device\r\n")
                .await;
            return;
        };
        let model = match Model::try_from(configspace.get_device_id()) {
            Ok(model) => model,
            Err(()) => {
                crate::VGA
                    .print_str_async(&format!(
                        "Unknown intel pro/1000 device id {:x}\r\n",
                        configspace.get_device_id()
                    ))
                    .await;
                return;
            }
        };
        let irqnum = match config {
            ConfigurationSpaceEnum::Standard(configuration_space_standard) => {
                configuration_space_standard.get_interrupt_line()
            }
            _ => {
                crate::VGA
                    .print_str_async(
                        "Intel pro/1000 on a non standard configuration space, not binding\r\n",
                    )
                    .await;
                return;
            }
        };
        let com = IrqGuardedInner::new(IrqNumbers::Only1(irqnum), false, true, |_| {}, |_| {});
        let m = IrqGuarded::new(bar0, &com);
        let up = AtomicBool::new(false);
        let ps = crate::common::new_irq_stream(&com, 10, 10);
        let mut d = IntelPro1000Device {
            internal: Arc::new(IntelPro1000DeviceInternal::new(m, up, &com)),
            _bars: bars,
            _io: io,
            eeprom_present: None,
            txbufs: None,
            txbufindex: 0,
            model,
            mac_address: MacAddress::default(),
            phy_address: None,
            tx_errors: AtomicU32::new(0),
            recvr: ps.0,
            sendr: ps.1,
        };
        f.set_bus_mastering(cs, bus, dev, true);
        crate::VGA
            .print_str_async(&format!("Detected model as {:?}\r\n", d.model))
            .await;
        crate::VGA
            .print_str_async(&format!("The irq line is {}\r\n", irqnum))
            .await;
        // The datasheet order: reset, general configuration, then the transmit and receive
        // configuration, and only then interrupts. Configuring the device while its interrupts are
        // live only gives the handler work to do on half configured rings.
        d.reset().await;
        d.general_config().await;
        crate::VGA
            .print_str_async(&format!("EEPROM DETECTED: {}\r\n", d.detect_eeprom().await))
            .await;
        let mac = d.get_mac_address().await;
        crate::VGA
            .print_str_async(&format!("The mac address is {:x?}\r\n", mac))
            .await;
        if let Err(e) = d.init_rx(&mac).await {
            // Registering a device that cannot receive would hand the network stack an interface
            // that silently does nothing.
            crate::VGA
                .print_str_async(&format!(
                    "RX buffer allocation error {:?}, not registering the device\r\n",
                    e
                ))
                .await;
            return;
        }
        if let Err(e) = d.init_tx().await {
            crate::VGA
                .print_str_async(&format!(
                    "TX buffer allocation error {:?}, not registering the device\r\n",
                    e
                ))
                .await;
            return;
        }
        {
            let sys = crate::SYSTEM.read();
            d.enable_interrupts(&sys, irqnum).await;
        }
        if crate::DEBUG_PRINT.load(Ordering::SeqCst) {
            let bar0 = d.internal.bar0.access().await;
            bar0.hex_dump().await;
        }
        d.dump_registers().await;
        d.internal.update_link_status().await;
        crate::VGA
            .print_str_async(&format!(
                "The link is up {}\r\n",
                d.internal.up.load(Ordering::Relaxed)
            ))
            .await;
        d.detect_phy_address().await;
        let phy = d.phy_address;
        if let Some(phy) = phy {
            net_print!("Found phy at address {}\r\n", phy);
            for r in 0..32 {
                if let Some(v) = d.read_from_phy(phy, r).await {
                    net_print!("PHY {} reg {} is {:x}\r\n", phy, r, v);
                }
            }
        } else {
            crate::VGA
                .print_str_async("No phy responded on the mdi bus\r\n")
                .await;
        }
        super::super::register_network_adapter(d.into()).await;
    }
}

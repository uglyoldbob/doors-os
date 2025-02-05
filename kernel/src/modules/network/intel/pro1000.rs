//! This driver is for the intel pro/1000 networking hardware

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
    Memory(crate::PciMemory),
    Io(crate::IoPortArray<'static>),
}

#[derive(Debug)]
enum Model {
    Model82540EP_A_Desktop,
    Model82540EP_A_Mobile,
    Model82540EM_A_Desktop,
    Model82540EM_A_Mobile,
    Model82541EI_A0_or_Model82541EI_B0_Copper,
    Model82541EI_B0_Mobile,
    Model82541GI_B1_Copper_or_Model82541PI_C0,
    Model82541GI_B1_Mobile,
    Model82541PI_C0,
    Model82544EI_A4,
    Model82544GC_A4,
    Model82545EM_A_Copper,
    Model82545EM_A_Fiber,
    Model82545GM_B_Copper,
    Model82545GM_B_Fiber,
    Model82545GM_B_SerDes,
    Model82546EB_A1_CopperDual,
    Model82546EB_A1_Fiber,
    Model82546EB_A1_CopperQuad,
    Model82546GB_B0_Copper,
    Model82546GB_B0_Fiber,
    Model82546GB_B0_SerDes,
    Model82547EI_A0_or_Model82547EI_A1_or_Model82547EI_B0_Copper_or_Model82547GI_B0,
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

    fn read(&self, address: u16) -> u32 {
        match self {
            MemoryOrIo::Memory(mem) => mem.read_u32(address as usize),
            MemoryOrIo::Io(io) => {
                let mut iop: crate::IoPortRef<u32> = io.port(address);
                iop.port_read()
            }
        }
    }

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

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u16)]
enum IntelPro1000Registers {
    Eeprom = 0x14,
    Rctrl = 0x100,
    Tctrl = 0x400,
    RxDescLow = 0x2800,
    RxDescHigh = 0x2804,
    RxDescLen = 0x2808,
    RxDescHead = 0x2810,
    RxDescTail = 0x2818,
    TxDescLow = 0x3800,
    TxDescHigh = 0x3804,
    TxDescLen = 0x3808,
    TxDescHead = 0x3810,
    TxDescTail = 0x3818,
}

/// Ethernet driver for the intel pro/1000 ethernet controller on pci
/// TODO: move this to crate::modules::network
#[derive(Clone, Default)]
pub struct IntelPro1000 {}

#[repr(C, packed)]
struct RxBuffer {
    address: u64,
    length: u16,
    checksum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

impl RxBuffer {
    fn new() -> Self {
        let buf: alloc::boxed::Box<[u8; 8192]> = alloc::boxed::Box::new([0; 8192]);
        let buf2 = alloc::boxed::Box::leak(buf);
        Self {
            address: crate::slice_address(buf2) as u64,
            length: 0,
            checksum: 0,
            status: 0,
            errors: 0,
            special: 0,
        }
    }
}

#[repr(C, packed)]
struct TxBuffer {
    address: u64,
    length: u16,
    cso: u8,
    cmd: u8,
    status: u8,
    css: u8,
    special: u16,
}

impl TxBuffer {
    fn new() -> Self {
        let buf: alloc::boxed::Box<[u8; 8192]> = alloc::boxed::Box::new([0; 8192]);
        let buf2 = alloc::boxed::Box::leak(buf);
        Self {
            address: crate::slice_address(buf2) as u64,
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
    fn new(quantity: u8, size: usize) -> Result<Self, core::alloc::AllocError> {
        let m: crate::DmaMemorySlice<RxBuffer> =
            crate::DmaMemorySlice::new_with(quantity as usize, |_| Ok(RxBuffer::new()))?;
        let mut dmas = alloc::vec::Vec::with_capacity(quantity as usize);
        for _i in 0..quantity {
            dmas.push(crate::DmaMemorySlice::new(size)?);
        }
        Ok(Self { bufs: m, dmas })
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
    fn new(quantity: u8, size: usize) -> Result<Self, core::alloc::AllocError> {
        let m: crate::DmaMemorySlice<TxBuffer> =
            crate::DmaMemorySlice::new_with(quantity as usize, |_| Ok(TxBuffer::new()))?;
        let mut dmas = alloc::vec::Vec::with_capacity(quantity as usize);
        for _i in 0..quantity {
            dmas.push(crate::DmaMemorySlice::new(size)?);
        }
        Ok(Self { bufs: m, dmas })
    }
}

/// The actual intel pro/1000 device
#[doors_macros::enum_variant(NetworkAdapter)]
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

const RCTRL_EN: u32 = 1 << 1;
const RCTRL_SBP: u32 = 1 << 2;
const RCTRL_UPE: u32 = 1 << 3;
const RCTRL_MPE: u32 = 1 << 4;
const RCTRL_LBM_NONE: u32 = 0;
const RCTRL_RDMTS_HALF: u32 = 0;
const RCTRL_BAM: u32 = 1 << 15;
const RCTRL_SECRC: u32 = 1 << 26;
const RCTRL_BSIZE_8192: u32 = 2 << 16 | 1 << 25;

const TCTRL_EN: u32 = 1 << 1;
const TCTRL_PSP: u32 = 1 << 3;
const TCTRL_CT_SHIFT: u8 = 4;
const TCTRL_COLD_SHIFT: u8 = 12u8;
const TCTRL_RTLC: u32 = 1 << 24;

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

    fn supports_pcix(&self) -> bool {
        match self.model {
            Model::Model82541EI_A0_or_Model82541EI_B0_Copper
            | Model::Model82541EI_B0_Mobile
            | Model::Model82541GI_B1_Copper_or_Model82541PI_C0
            | Model::Model82541GI_B1_Mobile
            | Model::Model82541PI_C0
            | Model::Model82540EP_A_Desktop
            | Model::Model82540EP_A_Mobile
            | Model::Model82540EM_A_Desktop
            | Model::Model82540EM_A_Mobile => false,
            _ => true,
        }
    }

    fn init_rx(&mut self) -> Result<(), core::alloc::AllocError> {
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
                RCTRL_EN
                    | RCTRL_SBP
                    | RCTRL_UPE
                    | RCTRL_MPE
                    | RCTRL_LBM_NONE
                    | RCTRL_RDMTS_HALF
                    | RCTRL_BAM
                    | RCTRL_SECRC
                    | RCTRL_BSIZE_8192,
            );
            self.rxbufindex = Some(0);
            doors_macros2::kernel_print!("RX BUFFER ARRAY IS AT {:x}\r\n", rxaddr);
            for r in rxbuf.bufs.iter() {
                doors_macros2::kernel_print!("\tIndividual buffer addr is {:x}\r\n", unsafe {
                    core::ptr::read_unaligned(&raw const r.address)
                });
            }
            self.rxbufs = Some(rxbuf);
        }
        Ok(())
    }

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
                TCTRL_EN
                    | TCTRL_PSP
                    | (15 << TCTRL_CT_SHIFT)
                    | (64 << TCTRL_COLD_SHIFT)
                    | TCTRL_RTLC,
            );
            self.txbufindex = Some(0);
            doors_macros2::kernel_print!("TX BUFFER ARRAY IS AT {:x}\r\n", txaddr);
            for t in txbuf.bufs.iter() {
                doors_macros2::kernel_print!("\tIndividual buffer addr is {:x}\r\n", unsafe {
                    core::ptr::read_unaligned(&raw const t.address)
                });
            }
            self.txbufs = Some(txbuf);
        }
        Ok(())
    }

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
            let vendor_combo = (dev as u32) << 16 | 0x8086;
            if !m.contains_key(&vendor_combo) {
                m.insert(vendor_combo, self.clone().into());
            }
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
                        if let Some(io) = bar.get_io(cs, bus, dev, f, config) {
                            Some(MemoryOrIo::Io(io))
                        } else {
                            None
                        }
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
                for b in &bars {
                    if let Some(b) = b {
                        b.print();
                    }
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
                hex_dump(&d.get_mac_address().address, false, false);
                if let Err(e) = d.init_rx() {
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

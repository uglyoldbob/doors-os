//! x86 or x64 interrupt code

use alloc::collections::btree_map::BTreeMap;

use crate::{
    modules::interrupt::InterruptControllerTrait, IoReadWrite, IrqGuarded, IrqGuardedInner,
    IrqGuardedUse,
};

#[repr(C)]
struct LocalApicRegisters {
    registers: [u32; 256],
}

bitfield::bitfield! {
    /// The data used to configure an entry in the ioapic
    struct IoApicRedirection(u64);
    impl Debug;
    /// The destination for the interrupt
    u8, destination, set_destination: 63, 56;
    /// The interrupt should be masked
    mask, set_mask: 16;
    /// The trigger mode is level sensitive, false means edge sensitive
    mode, set_mode: 15;
    /// remote irr, set when the interrupt is sent, cleared when eoi is received by lapic
    remote_irr, _: 14;
    /// The polarity of the interrupt, true means low active
    polarity, set_polarity: 13;
    /// The delivery status, true means the interrupt send is pending for some reason
    delivery_status, _: 12;
    /// The destination is logical instead of physical when true
    destination_mode, set_destination_mode: 11;
    /// The delivery mode
    /// 0 - fixed - deliver to all processors listed with INTR
    /// 1 - lowest priority - deliver to processor running with lowest priority on INTR
    /// 2 - smi - requires edge trigger mode, vector must be 0 for future compatibility
    /// 3 - reserved
    /// 4 - nmi - requires edge trigger mode, ignores vector information, delivers nmi signal to all processor cores listed
    /// 5 - init - requires edge trigger mode, deliver an init signal to all processor cores listed
    /// 6 - reserved
    /// 7 - extint - deliver to an 8259 pic with INTR, requires edge mode trigger
    u8, delivery_mode, set_delivery_mode: 10, 8;
    /// The vector to deliver to
    u8, vector, set_vector: 7, 0;
    /// The lower 32 bits, stored in the first 4 bytes of the register for the ioapic
    u32, lower_half, _: 31, 0;
    /// the upper 32 bits, stored in the second 4 bytes of the register for the ioapic
    u32, upper_half, _: 63, 32;
}

/// The local apic struct
pub struct LocalApic {
    regs: IrqGuarded<&'static mut LocalApicRegisters>,
    ioapic: Option<IoApic>,
    pic: Option<Pic>,
}

impl super::InterruptControllerTrait for LocalApic {
    fn end_of_interrupt(&self, _num: u8) {
        self.regs.interrupt_access().registers[0x2c] = 0;
    }

    fn enable_irq_sync(&self, num: u8) {
        if let Some(ioapic) = &self.ioapic {
            if let Some(sysirq) = ioapic.overrides.get(&num) {
                let irq = *sysirq as u8;
                ioapic.enable_irq_sync(irq);
            } else {
                ioapic.enable_irq_sync(num);
            }
        }
    }

    fn lookup_irq_with_channel(&self, channel: u8) -> Option<u8> {
        if let Some(ioapic) = &self.ioapic {
            for a in &ioapic.overrides {
                if *a.1 == channel as u32 {
                    return Some(*a.0);
                }
            }
            Some(channel)
        } else {
            None
        }
    }

    fn enable_irq_interrupt(&self, num: u8) {
        if let Some(ioapic) = &self.ioapic {
            if let Some(sysirq) = ioapic.overrides.get(&num) {
                let irq = *sysirq as u8;
                ioapic.enable_irq_interrupt(irq);
            } else {
                ioapic.enable_irq_interrupt(num);
            }
        }
    }

    fn disable_irq_sync(&self, num: u8) {
        if let Some(ioapic) = &self.ioapic {
            if let Some(sysirq) = ioapic.overrides.get(&num) {
                let irq = *sysirq as u8;
                ioapic.disable_irq_sync(irq);
            } else {
                ioapic.disable_irq_sync(num);
            }
        }
    }

    fn disable_irq_interrupt(&self, num: u8) {
        if let Some(ioapic) = &self.ioapic {
            if let Some(sysirq) = ioapic.overrides.get(&num) {
                let irq = *sysirq as u8;
                ioapic.disable_irq_interrupt(irq);
            } else {
                ioapic.disable_irq_interrupt(num);
            }
        }
    }

    fn is_irq_enabled(&self, irq: u8) -> bool {
        false
    }
}

impl LocalApic {
    ///get the apic base address
    #[cfg(target_arch = "x86_64")]
    fn get_base() -> usize {
        x86_64::registers::model_specific::ApicBase::read()
            .0
            .start_address()
            .as_u64() as usize
    }

    #[cfg(target_arch = "x86")]
    fn get_base() -> usize {
        let a = unsafe { x86::msr::rdmsr(x86::msr::IA32_APIC_BASE) };
        a as usize
    }

    /// Register the io apic with the local apic
    pub fn register_ioapic(&mut self, ioapic: IoApic) {
        self.ioapic = Some(ioapic);
    }

    /// Register the original pic object
    pub fn register_pic(&mut self, pic: Pic) {
        for i in [0, 1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15] {
            if pic.is_irq_enabled(i) {
                pic.disable_irq_sync(i);
                self.enable_irq_sync(i);
            }
        }
        pic.disable_irq_sync(2);
        self.pic = Some(pic);
        self.regs.sync_access().registers[0x3c] |= 0x100;
        #[cfg(target_arch = "x86_64")]
        {
            let mut bv = x86_64::registers::model_specific::ApicBase::read();
            bv.1 |= x86_64::registers::model_specific::ApicBaseFlags::LAPIC_ENABLE;
            unsafe {
                x86_64::registers::model_specific::ApicBase::write(bv.0, bv.1);
            }
        }
    }

    /// print the irq enable disable map
    pub fn print(&self) {
        if let Some(ioapic) = &self.ioapic {
            for i in 0..24 {
                if let Some(sysirq) = ioapic.overrides.get(&i) {
                    let irq = *sysirq as u8;
                    crate::VGA.print_str(&alloc::format!(
                        "IRQ {} manipulates entry {} -> {}\r\n",
                        i,
                        irq,
                        ioapic.get_mapping(irq)
                    ));
                } else {
                    crate::VGA.print_str(&alloc::format!(
                        "IRQ {} manipulates entry {} -> {}\r\n",
                        i,
                        i,
                        ioapic.get_mapping(i)
                    ));
                }
            }
        }
    }

    /// construct a new self
    pub fn new() -> Self {
        let paddr = Self::get_base();
        let vm = crate::boot::x86::boot::VIRTUAL_MEMORY_ALLOCATOR
            .sync_lock()
            .allocate_nonram_memory(0x1000, 0x1000)
            .unwrap();
        let vaddr = crate::slice_address(unsafe { vm.as_ref() });
        crate::boot::x86::boot::PAGING_MANAGER
            .sync_lock()
            .map_addresses_read_write(vaddr, paddr, 0x1000)
            .unwrap();
        crate::VGA.print_str(&alloc::format!("lapic at {:x} {:x}\r\n", paddr, vaddr));
        let regs = unsafe { &mut *(vaddr as *mut LocalApicRegisters) };
        let com = IrqGuardedInner::new(crate::IrqNumbers::None, true, false, |_| {}, |_| {});
        let s = Self {
            regs: IrqGuarded::new(regs, &com),
            ioapic: None,
            pic: None,
        };
        s
    }
}

/// The io apic for x86
pub struct IoApicInner {
    reg_sel: &'static mut u8,
    data: &'static mut u32,
    last_register: u8,
    num_irq: u8,
}

/// The io apic for x86
pub struct IoApic {
    inner: crate::IrqGuarded<IoApicInner>,
    overrides: BTreeMap<u8, u32>,
}

enum InterruptMode {
    Logical { apic_id: u8 },
    Physical { processors: u8 },
}

impl IoApic {
    /// Construct a new module wit the specified base address for registers
    pub fn new(addr: usize) -> Self {
        let mut i = IoApicInner::new(addr);
        let com = IrqGuardedInner::new(crate::IrqNumbers::None, true, false, |_| {}, |_| {});
        let r = i.read_register(1);
        let max = (r >> 16) + 1;
        crate::VGA.print_str(&alloc::format!(
            "IOAPIC HAS {} entries with {:x}\r\n",
            max,
            r
        ));
        let mut s = Self {
            inner: IrqGuarded::new(i, &com),
            overrides: BTreeMap::new(),
        };
        for x in 0..max {
            s.map_irq(
                InterruptMode::Physical { processors: 0 },
                x as u8,
                32 + x as u8,
            );
        }
        s
    }

    /// Get the mapping for the irq
    fn get_mapping(&self, irq: u8) -> u8 {
        let mut this = self.inner.sync_access();
        let mut data: u64 = 0;
        let o1 = this.read_register(0x10 + 2 * irq);
        let o2 = this.read_register(0x10 + 2 * irq + 1);
        data |= o1 as u64;
        data |= (o2 as u64) << 32;
        let entry = IoApicRedirection(data);
        entry.vector()
    }

    fn map_irq(&mut self, mode: InterruptMode, irq: u8, dest: u8) {
        crate::VGA.print_str(&alloc::format!("IOAPIC MAT {} to {}\r\n", irq, dest));
        let mut this = self.inner.interrupt_access();
        let mut data: u64 = 0;
        let o1 = this.read_register(0x10 + 2 * irq);
        let o2 = this.read_register(0x10 + 2 * irq + 1);
        data |= o1 as u64;
        data |= (o2 as u64) << 32;
        let mut entry = IoApicRedirection(data);
        match mode {
            InterruptMode::Logical { apic_id } => {
                entry.set_destination_mode(true);
                entry.set_destination(apic_id & 0xF);
            }
            InterruptMode::Physical { processors } => {
                entry.set_destination_mode(false);
                entry.set_destination(processors);
            }
        }
        entry.set_mask(true); //disable irq when mapping it
        entry.set_vector(dest);
        let db1 = entry.lower_half();
        let db2 = entry.upper_half();
        this.write_register(0x10 + 2 * irq, db1);
        this.write_register(0x10 + 2 * irq + 1, db2);
    }

    /// Register an interrupt override
    pub fn register_override(&mut self, irq: u8, sys_irq: u32) {
        if irq as u32 != sys_irq {
            self.overrides.insert(irq, sys_irq);
            self.map_irq(
                InterruptMode::Physical { processors: 0 },
                sys_irq as u8,
                32 + irq as u8,
            );
        }
    }

    /// common code for enabling an irq
    #[inline(never)]
    fn common_set_irq<T>(&self, irq: u8, val: bool, mut this: IrqGuardedUse<'_, IoApicInner, T>) {
        let mut data: u64 = 0;
        let o1 = this.read_register(0x10 + 2 * irq);
        let o2 = this.read_register(0x10 + 2 * irq + 1);
        data |= o1 as u64;
        data |= (o2 as u64) << 32;
        let mut entry = IoApicRedirection(data);
        entry.set_mask(val);
        this.write_register(0x10 + 2 * irq, entry.lower_half() as u32);
    }

    /// Enable the specified irq
    fn enable_irq_sync(&self, irq: u8) {
        let this = self.inner.sync_access();
        self.common_set_irq(irq, false, this);
    }

    /// Enable the specified irq from an interrupt
    fn enable_irq_interrupt(&self, irq: u8) {
        let this = self.inner.interrupt_access();
        self.common_set_irq(irq, false, this);
    }

    /// Disable the specified irq
    fn disable_irq_sync(&self, irq: u8) {
        let this = self.inner.sync_access();
        self.common_set_irq(irq, true, this);
    }

    /// Disable the specified irq
    fn disable_irq_interrupt(&self, irq: u8) {
        let this = self.inner.interrupt_access();
        self.common_set_irq(irq, true, this);
    }
}

impl IoApicInner {
    fn switch_registers(&mut self, index: u8) {
        if self.last_register != index {
            self.last_register = index;
            *self.reg_sel = index;
        }
    }

    fn read_register(&mut self, index: u8) -> u32 {
        self.switch_registers(index);
        *self.data
    }

    fn write_register(&mut self, index: u8, val: u32) {
        self.switch_registers(index);
        unsafe { core::ptr::write_volatile(self.data, val) };
    }

    /// Construct a new module wit the specified base address for registers
    pub fn new(addr: usize) -> Self {
        let mut s = Self {
            reg_sel: unsafe { &mut *(addr as *mut u8) },
            data: unsafe { &mut *((addr + 0x10) as *mut u32) },
            last_register: 3,
            num_irq: 24,
        };
        let r = s.read_register(1);
        let num_irq = (r >> 16) + 1;
        s.num_irq = num_irq as u8;
        s
    }
}

/// The programmable interrupt controller
pub struct Pic {
    /// The first pic
    pic1: crate::IoPortArray<'static>,
    /// The second pic
    pic2: crate::IoPortArray<'static>,
}

impl Pic {
    /// Get a pic object.
    pub fn new() -> Option<Self> {
        Some(Self {
            pic1: crate::IO_PORT_MANAGER
                .as_ref()
                .unwrap()
                .get_ports(0x20, 2)?,
            pic2: crate::IO_PORT_MANAGER
                .as_ref()
                .unwrap()
                .get_ports(0xa0, 2)?,
        })
    }

    /// Signal end of interrupt for the specified irq
    pub fn pic_end_of_interrupt(&self, irq: u8) {
        if irq >= 8 {
            self.pic2.port(0).port_write(0x20u8);
        }
        self.pic1.port(0).port_write(0x20u8);
    }

    /// Disable all interrupts for both pics
    pub fn disable(&self) {
        use crate::IoReadWrite;
        self.pic1.port(1).port_write(0xffu8);
        self.pic2.port(1).port_write(0xffu8);
    }

    /// Enable the specified irq
    #[inline(never)]
    pub fn pic_enable_irq(&self, irq: u8) {
        if irq < 8 {
            let data: u8 = self.pic1.port(1).port_read();
            self.pic1.port(1).port_write(data & !(1 << irq));
        } else {
            let irq = irq - 8;
            let data: u8 = self.pic2.port(1).port_read();
            self.pic2.port(1).port_write(data & !(1 << irq));
        }
    }

    /// Disable the specified irq
    #[inline(never)]
    pub fn pic_disable_irq(&self, irq: u8) {
        if irq < 8 {
            let data: u8 = self.pic1.port(1).port_read();
            self.pic1.port(1).port_write(data | (1 << irq));
        } else {
            let irq = irq - 8;
            let data: u8 = self.pic2.port(1).port_read();
            self.pic2.port(1).port_write(data | (1 << irq));
        }
    }

    /// Perform a remap of the pic interrupts
    /// # Arguments
    /// * offset1 - The amount to offset pic1 vectors by
    /// * offset2 - The amount to offset pic2 vectors by
    pub fn remap(&self, offset1: u8, offset2: u8) {
        use crate::IoReadWrite;
        let mut delay: crate::IoPortRef<u8> = crate::IO_PORT_MANAGER
            .as_ref()
            .unwrap()
            .get_port(0x80)
            .unwrap();

        let mut pic1_cmd: crate::IoPortRef<u8> = self.pic1.port(0);
        let mut pic1_data: crate::IoPortRef<u8> = self.pic1.port(1);
        let mut pic2_cmd: crate::IoPortRef<u8> = self.pic2.port(0);
        let mut pic2_data: crate::IoPortRef<u8> = self.pic2.port(1);

        let mask1 = pic1_data.port_read();
        let mask2 = pic2_data.port_read();
        pic1_cmd.port_write(0x11);
        delay.port_write(0);
        pic2_cmd.port_write(0x11);
        delay.port_write(0);
        pic1_data.port_write(offset1);
        delay.port_write(0);
        pic2_data.port_write(offset2);
        delay.port_write(0);
        pic1_data.port_write(4);
        delay.port_write(0);
        pic2_data.port_write(2);
        delay.port_write(0);
        pic1_data.port_write(1);
        delay.port_write(0);
        pic2_data.port_write(1);
        delay.port_write(0);

        pic1_data.port_write(mask1);
        pic2_data.port_write(mask2);
        self.disable();
        self.pic_enable_irq(2); //enable the interrupt for the second pic
    }
}

impl super::InterruptControllerTrait for Pic {
    fn end_of_interrupt(&self, num: u8) {
        self.pic_end_of_interrupt(num);
    }

    fn enable_irq_sync(&self, num: u8) {
        self.pic_enable_irq(num);
    }

    fn disable_irq_sync(&self, num: u8) {
        self.pic_disable_irq(num);
    }

    fn enable_irq_interrupt(&self, num: u8) {
        self.pic_enable_irq(num);
    }

    fn disable_irq_interrupt(&self, num: u8) {
        self.pic_disable_irq(num);
    }

    fn lookup_irq_with_channel(&self, channel: u8) -> Option<u8> {
        Some(channel)
    }

    fn is_irq_enabled(&self, irq: u8) -> bool {
        if irq < 8 {
            let data: u8 = self.pic1.port(1).port_read();
            ((data >> irq) & 1) == 0
        } else {
            let irq = irq - 8;
            let data: u8 = self.pic2.port(1).port_read();
            ((data >> irq) & 1) == 0
        }
    }
}

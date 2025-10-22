//! x86 or x64 interrupt code

use crate::IoReadWrite;

/// The io apic for x86
pub struct IoApic {
    reg_sel: &'static mut u8,
    data: &'static mut u32,
    last_register: u8,
    num_irq: u8,
}

impl super::InterruptControllerTrait for IoApic {
    fn end_of_interrupt(&self, num: u8) {}

    fn enable_irq(&self, num: u8) {}

    fn disable_irq(&self, num: u8) {}
}

impl IoApic {
    fn read_register(&mut self, index: u8) -> u32 {
        if self.last_register != index {
            self.last_register = index;
            *self.reg_sel = index;
        }
        *self.data
    }

    /// Construct a new module wit the specified base address for registers
    pub fn new(addr: usize) -> Self {
        let mut s = Self {
            reg_sel: unsafe { &mut *(addr as *mut u8) },
            data: unsafe { &mut *((addr + 0x10) as *mut u32) },
            last_register: 3,
            num_irq: 24,
        };
        crate::VGA.print_str(&alloc::format!(
            "IOAPIC ID is {:x}\r\n",
            (s.read_register(0) >> 24) & 0xf
        ));
        let num_irq = 1 + ((s.read_register(1) >> 16) & 0xFF) as u8;
        s.num_irq = num_irq;
        for i in 0..3 {
            crate::VGA.print_str(&alloc::format!("IOAPIC {:x}\r\n", i));
            crate::VGA.print_str(&alloc::format!("\t{:x}\r\n", s.read_register(i as u8)));
        }
        for i in 16..64 {
            crate::VGA.print_str(&alloc::format!("IOAPIC {:x}\r\n", i));
            crate::VGA.print_str(&alloc::format!("\t{:x}\r\n", s.read_register(i as u8)));
        }
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

impl Drop for Pic {
    fn drop(&mut self) {}
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

    fn enable_irq(&self, num: u8) {
        self.pic_enable_irq(num);
    }

    fn disable_irq(&self, num: u8) {
        self.pic_disable_irq(num);
    }
}

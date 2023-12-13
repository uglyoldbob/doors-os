//! Boot code for the stm32F769i-disco development board

use alloc::vec::Vec;

use crate::modules::gpio::GpioTrait;

pub mod memory;

extern "C" {
    /// Defines the start of the kernel for initial kernel load. This is defined by the linker script.
    pub static START_OF_BSS: u8;
    /// Defines the end of the kernel for the initial kernel load. This is defined by the linker script.
    pub static END_OF_BSS: u8;
    /// Defines the location for RAM data initial data
    pub static RAMLOAD: u8;
}

/// The entry point of the kernel
#[no_mangle]
pub extern "C" fn _start() -> ! {
    let start_bss = unsafe { &START_OF_BSS } as *const u8 as usize;
    let end_bss = unsafe { &END_OF_BSS } as *const u8 as usize;
    let bss = unsafe { core::slice::from_raw_parts_mut(start_bss as *mut u8, end_bss - start_bss) };
    for e in bss {
        *e = 0;
    }

    let ramload = unsafe { &RAMLOAD } as *const u8 as usize;
    let ram =
        unsafe { core::slice::from_raw_parts_mut(0x2002_0000 as *mut u8, start_bss - 0x2002_0000) };
    let ram_data =
        unsafe { core::slice::from_raw_parts(ramload as *mut u8, start_bss - 0x2002_0000) };
    ram.clone_from_slice(ram_data);

    let mut h = super::HEAP_MANAGER.lock();
    if end_bss < 0x2007_c000 {
        let a = core::mem::align_of::<memory::HeapNode>() - 1;
        let end = if (end_bss & a) > 0 {
            let m = end_bss & !a;
            m + a + 1
        } else {
            end_bss
        };
        h.init(0, end, 0x2007_c000 - end);
    }
    drop(h);

    let mut rcc = unsafe { crate::modules::reset::stm32f769::Module::new(0x4002_3800) };
    rcc.enable_peripheral(9);
    rcc.enable_peripheral(0);

    let mut ga = unsafe { crate::modules::gpio::stm32f769::Gpio::new(0x4002_0000) };
    let mut gb = unsafe { crate::modules::gpio::stm32f769::Gpio::new(0x4002_0400) };
    let mut gc = unsafe { crate::modules::gpio::stm32f769::Gpio::new(0x4002_0800) };
    let mut gd = unsafe { crate::modules::gpio::stm32f769::Gpio::new(0x4002_0c00) };
    let mut ge = unsafe { crate::modules::gpio::stm32f769::Gpio::new(0x4002_1000) };
    let mut gf = unsafe { crate::modules::gpio::stm32f769::Gpio::new(0x4002_1400) };
    let mut gg = unsafe { crate::modules::gpio::stm32f769::Gpio::new(0x4002_1800) };
    let mut gh = unsafe { crate::modules::gpio::stm32f769::Gpio::new(0x4002_1c00) };
    let mut gi = unsafe { crate::modules::gpio::stm32f769::Gpio::new(0x4002_2000) };
    let mut gj = unsafe { crate::modules::gpio::stm32f769::Gpio::new(0x4002_2400) };
    let mut gk = unsafe { crate::modules::gpio::stm32f769::Gpio::new(0x4002_2800) };

    if true {
        let mut gpio = crate::kernel::GPIO.lock();
        gpio.register_gpios(ga.into());
        gpio.register_gpios(gb.into());
        gpio.register_gpios(gc.into());
        gpio.register_gpios(gd.into());
        gpio.register_gpios(ge.into());
        gpio.register_gpios(gf.into());
        gpio.register_gpios(gg.into());
        gpio.register_gpios(gh.into());
        gpio.register_gpios(gi.into());
        gpio.register_gpios(gj.into());
        gpio.register_gpios(gk.into());
        drop(gpio);
    }

    {
        let mut gpio = crate::kernel::GPIO.lock();
        let g = gpio.module(0);
        g.set_output(12);
        drop(g);

        let h = gpio.module(9);
        h.set_output(5);
        h.set_output(13);
        drop(h);
        loop {
            let g = gpio.module(0);
            g.write_output(12, true);
            drop(g);
            let h = gpio.module(9);
            h.write_output(5, true);
            h.write_output(13, true);
            drop(h);
            let g = gpio.module(0);
            g.write_output(12, false);
            drop(g);
            let h = gpio.module(9);
            h.write_output(5, false);
            h.write_output(13, false);
            drop(h);
        }
        drop(gpio);
    }
    crate::main()
}

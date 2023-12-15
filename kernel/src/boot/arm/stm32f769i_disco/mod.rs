//! Boot code for the stm32F769i-disco development board

use alloc::sync::Arc;

use crate::LockedArc;

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

    let rcc_mod = unsafe { crate::modules::reset::stm32f769::Module::new(0x4002_3800) };
    let rcc_mod = LockedArc::new(rcc_mod);
    let rcc = crate::modules::clock::ClockProvider::Stm32f769(rcc_mod.clone());

    let exto =
        unsafe { crate::modules::clock::stm32f769::ExternalOscillator::new(25_000_000, &rcc_mod) };
    let into =
        unsafe { crate::modules::clock::stm32f769::InternalOscillator::new(16_000_000, &rcc_mod) };
    crate::modules::clock::ClockProviderTrait::enable(&exto, 0);

    while !crate::modules::clock::ClockProviderTrait::is_ready(&exto, 0) {}

    let exto = crate::modules::clock::ClockProvider::Stm32f769Hse(exto).get_ref(0);
    let into = crate::modules::clock::ClockProvider::Stm32f769Hsi(into).get_ref(0);
    let mux1 = crate::modules::clock::stm32f769::Mux1::new(
        &rcc_mod,
        [alloc::boxed::Box::new(into), alloc::boxed::Box::new(exto)],
    );
    crate::modules::clock::ClockMuxTrait::select(&mux1, 1);

    let ga = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&rcc, 0, 0x4002_0000) };
    let gb = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&rcc, 1, 0x4002_0400) };
    let gc = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&rcc, 2, 0x4002_0800) };
    let gd = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&rcc, 3, 0x4002_0c00) };
    let ge = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&rcc, 4, 0x4002_1000) };
    let gf = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&rcc, 5, 0x4002_1400) };
    let gg = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&rcc, 6, 0x4002_1800) };
    let gh = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&rcc, 7, 0x4002_1c00) };
    let gi = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&rcc, 8, 0x4002_2000) };
    let gj = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&rcc, 9, 0x4002_2400) };
    let gk = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&rcc, 10, 0x4002_2800) };

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

    let dsi = unsafe { crate::modules::video::mipi_dsi::stm32f769::Module::new(&rcc, 0x4001_6c00) };
    crate::modules::video::mipi_dsi::MipiDsiTrait::enable(&dsi);
    crate::main()
}

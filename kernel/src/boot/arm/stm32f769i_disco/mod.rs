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

    // the power interface - move to a dedicated module
    crate::modules::clock::ClockProviderTrait::enable_clock(&rcc, 3 * 32 + 28);

    let power = unsafe { crate::modules::power::stm32f769::Power::new(0x4000_7000) };
    //set vos for main power? (with power object)

    let mut r = rcc_mod.lock();
    r.apb_dividers(4, 2);
    r.set_hse_bypass(true);
    drop(r);

    let mut fmc = unsafe { crate::modules::memory::stm32f769::Fmc::new(0x4002_3c00) };

    let exto =
        unsafe { crate::modules::clock::stm32f769::ExternalOscillator::new(25_000_000, &rcc_mod) };
    let into =
        unsafe { crate::modules::clock::stm32f769::InternalOscillator::new(16_000_000, &rcc_mod) };
    crate::modules::clock::ClockProviderTrait::enable_clock(&exto, 0);

    while !crate::modules::clock::ClockProviderTrait::clock_is_ready(&exto, 0) {}

    let exto = crate::modules::clock::ClockProvider::Stm32f769Hse(exto).get_ref(0);
    let into = crate::modules::clock::ClockProvider::Stm32f769Hsi(into).get_ref(0);
    let mux1 = crate::modules::clock::stm32f769::Mux1::new(
        &rcc_mod,
        [
            alloc::boxed::Box::new(into.clone()),
            alloc::boxed::Box::new(exto.clone()),
        ],
    );
    crate::modules::clock::ClockMuxTrait::select(&mux1, 1);

    let mux1 = crate::modules::clock::ClockMux::Stm32f769Mux1(mux1);
    let mux1 = crate::modules::clock::ClockRef::Mux(mux1);
    let divider = crate::modules::clock::stm32f769::Divider1::new(&rcc_mod, mux1.clone());
    divider.set_divider(25);

    let divider = crate::modules::clock::ClockRef::Stm32f769MainDivider(divider);

    let pll_main = crate::modules::clock::stm32f769::PllMain::new(&rcc_mod, divider.clone());
    let pll_main_provider =
        crate::modules::clock::ClockProvider::Stm32F769MainPll(pll_main.clone());
    let pll_main = crate::modules::clock::PllProvider::Stm32f769MainPll(pll_main.clone());
    let pll_two = crate::modules::clock::stm32f769::PllTwo::new(&rcc_mod, divider.clone());
    let pll_two_provider =
        crate::modules::clock::ClockProvider::Stm32F769SecondPll(pll_two.clone());
    let pll_two = crate::modules::clock::PllProvider::Stm32f769SecondPll(pll_two);
    let pll_three = crate::modules::clock::stm32f769::PllThree::new(&rcc_mod, divider.clone());
    let pll_three_provider =
        crate::modules::clock::ClockProvider::Stm32F769ThirdPll(pll_three.clone());
    let pll_three = crate::modules::clock::PllProvider::Stm32f769ThirdPll(pll_three);

    loop {
        if crate::modules::clock::PllProviderTrait::set_vco_frequency(&pll_main, 432_000_000)
            .is_ok()
        {
            break;
        }
    }

    let mut r = rcc_mod.lock();
    r.set_mco1_pll();
    drop(r);

    loop {
        if crate::modules::clock::PllProviderTrait::set_vco_frequency(&pll_two, 432_000_000).is_ok()
        {
            break;
        }
    }

    loop {
        if crate::modules::clock::PllProviderTrait::set_vco_frequency(&pll_three, 432_000_000)
            .is_ok()
        {
            break;
        }
    }

    crate::modules::clock::PllProviderTrait::set_post_divider(&pll_main, 0, 2);

    fmc.set_wait_states(6);

    crate::modules::clock::PllProviderTrait::set_post_divider(&pll_main, 2, 2);
    crate::modules::clock::ClockProviderTrait::enable_clock(&pll_main_provider, 0);
    while !crate::modules::clock::ClockProviderTrait::clock_is_ready(&pll_main_provider, 0) {}

    let pll_ref = pll_main_provider.get_ref(0);

    let sysclk_mux = crate::modules::clock::stm32f769::MuxSysClk::new(
        &rcc_mod,
        [
            alloc::boxed::Box::new(into),
            alloc::boxed::Box::new(exto),
            alloc::boxed::Box::new(pll_ref),
        ],
    );

    crate::modules::clock::ClockMuxTrait::select(&sysclk_mux, 2);

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

    let dsi_clock1 = mux1.clone();

    crate::modules::clock::PllProviderTrait::set_post_divider(&pll_three, 2, 2);
    crate::modules::clock::ClockProviderTrait::enable_clock(&pll_three, 0);
    while !crate::modules::clock::ClockProviderTrait::clock_is_ready(&pll_three, 0) {}

    let dsi_byte_clock = pll_main_provider.get_ref(2);

    let dsi = unsafe {
        crate::modules::video::mipi_dsi::stm32f769::Module::new(
            &rcc,
            [&dsi_byte_clock, &dsi_clock1],
            0x4001_6c00,
        )
    };

    let dsi_config = crate::modules::video::mipi_dsi::MipiDsiConfig {
        link_speed: 400_000_000,
        num_lanes: 2,
        vcid: 0,
    };

    let resolution = crate::modules::video::ScreenResolution {
        width: 800,
        height: 480,
        hsync: 7,
        vsync: 3,
        h_b_porch: 7,
        h_f_porch: 6,
        v_b_porch: 2,
        v_f_porch: 2,
    };

    crate::modules::video::mipi_dsi::MipiDsiTrait::enable(&dsi, dsi_config, resolution);

    crate::main()
}

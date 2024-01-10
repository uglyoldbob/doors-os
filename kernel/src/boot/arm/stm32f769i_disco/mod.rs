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

use crate::modules::gpio::GpioTrait;

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

    let osc32 = crate::modules::clock::FixedClock::new(None);
    let oscmain = crate::modules::clock::FixedClock::new(Some(25_000_000));
    let oscint = crate::modules::clock::FixedClock::new(Some(16_000_000));
    let osc32int = crate::modules::clock::FixedClock::new(Some(32_000));

    let rcc_mod = unsafe { crate::modules::reset::stm32f769::Module::new(0x4002_3800) };
    let rcc_mod = LockedArc::new(rcc_mod);

    let mut ctree = crate::modules::clock::stm32f769::ClockTree::new(
        osc32.into(),
        oscmain.into(),
        oscint.into(),
        osc32int.into(),
        rcc_mod.clone(),
    );

    let ctree = LockedArc::new(ctree);

    let ctree_provider = crate::modules::clock::ClockProvider::Stm32f769Provider(ctree.clone());
    let ctree_pll = crate::modules::clock::PllProvider::Stm32f769(ctree.clone());

    // enable the power interface
    //crate::modules::clock::ClockProviderTrait::enable_clock(&ctree, 4 * 32 + 28);

    //let power = unsafe { crate::modules::power::stm32f769::Power::new(0x4000_7000) };
    //set vos for main power? (with power object)

    let mut r = rcc_mod.lock();
    r.apb_dividers(4, 2);
    r.set_hse_bypass(true);
    drop(r);

    let mut fmc = unsafe { crate::modules::memory::stm32f769::Fmc::new(0x4002_3c00) };

    // enable the external oscillator
    crate::modules::clock::ClockProviderTrait::enable_clock(&ctree, 0);
    while !crate::modules::clock::ClockProviderTrait::clock_is_ready(&ctree, 0) {}

    //setup all three main pll of the system

    crate::modules::clock::PllProviderTrait::run_closure(&ctree_pll, 0, &|pll| {
        use crate::modules::clock::PllTrait;
        pll.get_input_frequency();
    });

    fmc.set_wait_states(6);

    let ga = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 0, 0x4002_0000) };
    let gb = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 1, 0x4002_0400) };
    let gc = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 2, 0x4002_0800) };
    let gd = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 3, 0x4002_0c00) };
    let ge = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 4, 0x4002_1000) };
    let gf = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 5, 0x4002_1400) };
    let gg = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 6, 0x4002_1800) };
    let gh = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 7, 0x4002_1c00) };
    let gi = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 8, 0x4002_2000) };
    let gj = unsafe { crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 9, 0x4002_2400) };
    let gk =
        unsafe { crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 10, 0x4002_2800) };

    {
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

    crate::main()
}

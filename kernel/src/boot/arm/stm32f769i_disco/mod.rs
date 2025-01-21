//! Boot code for the stm32F769i-disco development board

use crate::modules::video::mipi_dsi::MipiDsiTrait;
use crate::modules::video::TextDisplay;
use crate::{kernel, LockedArc};

pub mod memory;

extern "C" {
    /// Defines the start of the kernel for initial kernel load. This is defined by the linker script.
    pub static START_OF_BSS: u8;
    /// Defines the end of the kernel for the initial kernel load. This is defined by the linker script.
    pub static END_OF_BSS: u8;
    /// Defines the location for RAM data initial data
    pub static RAMLOAD: u8;
}

/// The definition of the isr table, starting at the nmi handler.
#[allow(dead_code)]
struct IsrTable {
    nmi: extern "C" fn(),
    hardfault: extern "C" fn(),
    memory: extern "C" fn(),
    bus_fault: extern "C" fn(),
    use_fault: extern "C" fn(),
    reserved1_1: u32,
    reserved1_2: u32,
    reserved1_3: u32,
    reserved1_4: u32,
    service: extern "C" fn(),
    debug: extern "C" fn(),
    reserved2: u32,
    pending: extern "C" fn(),
    systick: extern "C" fn(),
    watchdog: extern "C" fn(),
    pvd: extern "C" fn(),
    tamp_stamp: extern "C" fn(),
    rtc_wakeup: extern "C" fn(),
    flash: extern "C" fn(),
    rcc: extern "C" fn(),
    exti0: extern "C" fn(),
    exti1: extern "C" fn(),
    exti2: extern "C" fn(),
    exti3: extern "C" fn(),
    exti4: extern "C" fn(),
    dma1_0: extern "C" fn(),
    dma1_1: extern "C" fn(),
    dma1_2: extern "C" fn(),
    dma1_3: extern "C" fn(),
    dma1_4: extern "C" fn(),
    dma1_5: extern "C" fn(),
    dma1_6: extern "C" fn(),
    adc: extern "C" fn(),
    can1_tx: extern "C" fn(),
    can1_rx0: extern "C" fn(),
    can1_rx1: extern "C" fn(),
    can1_sce: extern "C" fn(),
    exti5_9: extern "C" fn(),
    tim1_9_brk: extern "C" fn(),
    tim1_10_update: extern "C" fn(),
    tim1_11_trigger_commutation: extern "C" fn(),
    tim1_cc: extern "C" fn(),
    tim2: extern "C" fn(),
    tim3: extern "C" fn(),
    tim4: extern "C" fn(),
    i2c1_event: extern "C" fn(),
    i2c1_err: extern "C" fn(),
    i2c2_event: extern "C" fn(),
    i2c2_err: extern "C" fn(),
    spi1: extern "C" fn(),
    spi2: extern "C" fn(),
    usart1: extern "C" fn(),
    usart2: extern "C" fn(),
    usart3: extern "C" fn(),
    exti10_15: extern "C" fn(),
    rtc_alarm: extern "C" fn(),
    usb_otg_fs_wakeup: extern "C" fn(),
    tim8_12_break: extern "C" fn(),
    tim8_13_update: extern "C" fn(),
    tim8_14_trigger_commutation: extern "C" fn(),
    tim8_cc: extern "C" fn(),
    dma1_7: extern "C" fn(),
    fmc: extern "C" fn(),
    sdmmc1: extern "C" fn(),
    tim5: extern "C" fn(),
    spi3: extern "C" fn(),
    uart4: extern "C" fn(),
    uart5: extern "C" fn(),
    tim6_dac: extern "C" fn(),
    tim7: extern "C" fn(),
    dma2_0: extern "C" fn(),
    dma2_1: extern "C" fn(),
    dma2_2: extern "C" fn(),
    dma2_3: extern "C" fn(),
    dma2_4: extern "C" fn(),
    eth: extern "C" fn(),
    eth_wakeup: extern "C" fn(),
    can2_tx: extern "C" fn(),
    can2_rx0: extern "C" fn(),
    can2_tx0: extern "C" fn(),
    can2_sce: extern "C" fn(),
    usb_otg_fs: extern "C" fn(),
    dma2_5: extern "C" fn(),
    dma2_6: extern "C" fn(),
    dma2_7: extern "C" fn(),
    usart6: extern "C" fn(),
    i2c3_event: extern "C" fn(),
    i2c3_error: extern "C" fn(),
    usb_otg_ep1_out: extern "C" fn(),
    usb_otg_ep1_in: extern "C" fn(),
    usb_otg_hs_wakeup: extern "C" fn(),
    usb_otg_hs: extern "C" fn(),
    dcmi: extern "C" fn(),
    cryp: extern "C" fn(),
    hash_rng: extern "C" fn(),
    fpu: extern "C" fn(),
    uart7: extern "C" fn(),
    uart8: extern "C" fn(),
    spi4: extern "C" fn(),
    spi5: extern "C" fn(),
    spi6: extern "C" fn(),
    sai1: extern "C" fn(),
    lcd_tft: extern "C" fn(),
    lcd_tft_error: extern "C" fn(),
    dma2d: extern "C" fn(),
    quadspi: extern "C" fn(),
    lp_tim1: extern "C" fn(),
    hdmi_cec: extern "C" fn(),
    i2c4_event: extern "C" fn(),
    i2c4_error: extern "C" fn(),
    spdifrx: extern "C" fn(),
    dsi: extern "C" fn(),
    dfsdm1_0: extern "C" fn(),
    dfsdm1_1: extern "C" fn(),
    dfsdm1_2: extern "C" fn(),
    dfsdm1_3: extern "C" fn(),
    sdmmc2: extern "C" fn(),
    can3_tx: extern "C" fn(),
    can3_rx0: extern "C" fn(),
    can3_rx1: extern "C" fn(),
    can3_sce: extern "C" fn(),
    jpeg: extern "C" fn(),
    mdios: extern "C" fn(),
}

impl IsrTable {
    const fn build() -> Self {
        Self {
            nmi: nmi_handler,
            hardfault: default_handler,
            memory: default_handler,
            bus_fault: default_handler,
            use_fault: default_handler,
            reserved1_1: 0,
            reserved1_2: 0,
            reserved1_3: 0,
            reserved1_4: 0,
            service: default_handler,
            debug: default_handler,
            reserved2: 0,
            pending: default_handler,
            systick: default_handler,
            watchdog: default_handler,
            pvd: default_handler,
            tamp_stamp: default_handler,
            rtc_wakeup: default_handler,
            flash: default_handler,
            rcc: default_handler,
            exti0: default_handler,
            exti1: default_handler,
            exti2: default_handler,
            exti3: default_handler,
            exti4: default_handler,
            dma1_0: default_handler,
            dma1_1: default_handler,
            dma1_2: default_handler,
            dma1_3: default_handler,
            dma1_4: default_handler,
            dma1_5: default_handler,
            dma1_6: default_handler,
            adc: default_handler,
            can1_tx: default_handler,
            can1_rx0: default_handler,
            can1_rx1: default_handler,
            can1_sce: default_handler,
            exti5_9: default_handler,
            tim1_9_brk: default_handler,
            tim1_10_update: default_handler,
            tim1_11_trigger_commutation: default_handler,
            tim1_cc: default_handler,
            tim2: default_handler,
            tim3: default_handler,
            tim4: default_handler,
            i2c1_event: default_handler,
            i2c1_err: default_handler,
            i2c2_event: default_handler,
            i2c2_err: default_handler,
            spi1: default_handler,
            spi2: default_handler,
            usart1: default_handler,
            usart2: default_handler,
            usart3: default_handler,
            exti10_15: default_handler,
            rtc_alarm: default_handler,
            usb_otg_fs_wakeup: default_handler,
            tim8_12_break: default_handler,
            tim8_13_update: default_handler,
            tim8_14_trigger_commutation: default_handler,
            tim8_cc: default_handler,
            dma1_7: default_handler,
            fmc: default_handler,
            sdmmc1: default_handler,
            tim5: default_handler,
            spi3: default_handler,
            uart4: default_handler,
            uart5: default_handler,
            tim6_dac: default_handler,
            tim7: default_handler,
            dma2_0: default_handler,
            dma2_1: default_handler,
            dma2_2: default_handler,
            dma2_3: default_handler,
            dma2_4: default_handler,
            eth: default_handler,
            eth_wakeup: default_handler,
            can2_tx: default_handler,
            can2_rx0: default_handler,
            can2_tx0: default_handler,
            can2_sce: default_handler,
            usb_otg_fs: default_handler,
            dma2_5: default_handler,
            dma2_6: default_handler,
            dma2_7: default_handler,
            usart6: default_handler,
            i2c3_event: default_handler,
            i2c3_error: default_handler,
            usb_otg_ep1_out: default_handler,
            usb_otg_ep1_in: default_handler,
            usb_otg_hs_wakeup: default_handler,
            usb_otg_hs: default_handler,
            dcmi: default_handler,
            cryp: default_handler,
            hash_rng: default_handler,
            fpu: default_handler,
            uart7: default_handler,
            uart8: default_handler,
            spi4: default_handler,
            spi5: default_handler,
            spi6: default_handler,
            sai1: default_handler,
            lcd_tft: default_handler,
            lcd_tft_error: default_handler,
            dma2d: default_handler,
            quadspi: default_handler,
            lp_tim1: default_handler,
            hdmi_cec: default_handler,
            i2c4_event: default_handler,
            i2c4_error: default_handler,
            spdifrx: default_handler,
            dsi: default_handler,
            dfsdm1_0: default_handler,
            dfsdm1_1: default_handler,
            dfsdm1_2: default_handler,
            dfsdm1_3: default_handler,
            sdmmc2: default_handler,
            can3_tx: default_handler,
            can3_rx0: default_handler,
            can3_rx1: default_handler,
            can3_sce: default_handler,
            jpeg: default_handler,
            mdios: default_handler,
        }
    }
}

/// The isr table, starting at the NMI handler
#[used]
#[link_section = ".isr_vector"]
static ISR_TABLE: IsrTable = IsrTable::build();

use crate::modules::clock::{ClockProviderTrait, PllProviderTrait, PllTrait};
use crate::modules::gpio::GpioTrait;

/// The nmi handler
pub extern "C" fn nmi_handler() {
    loop {}
}

/// The default interrupt handler
pub extern "C" fn default_handler() {
    loop {}
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

    let osc32 = crate::modules::clock::FixedClock::new(None);
    let oscmain = crate::modules::clock::FixedClock::new(Some(25_000_000));
    let oscint = crate::modules::clock::FixedClock::new(Some(16_000_000));
    let osc32int = crate::modules::clock::FixedClock::new(Some(32_000));

    let rcc_mod = unsafe { crate::modules::reset::stm32f769::Module::new(0x4002_3800) };
    let rcc_mod = LockedArc::new(rcc_mod);

    let ctree = crate::modules::clock::stm32f769::ClockTree::new(
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

    let mut fic =
        unsafe { crate::modules::memory::stm32f769::FlashInterfaceController::new(0x4002_3c00) };

    // enable the external oscillator
    crate::modules::clock::ClockProviderTrait::enable_clock(&ctree, 0);
    while !crate::modules::clock::ClockProviderTrait::clock_is_ready(&ctree, 0) {}

    //setup all three main pll of the system

    //setup the mco clock output
    let mut r = rcc_mod.lock();
    r.set_mco1_pll();
    drop(r);

    let mut c = ctree.lock();
    c.mux1_select(1); //select the external oscillator
    c.divider1_set(25); //divide down to a 1 mhz clock
    drop(c);
    let pllsetup = crate::modules::clock::PllProviderTrait::run_closure(&ctree_pll, 0, &|pll| {
        if pll.set_input_divider(1).is_ok()
            && pll.set_vco_frequency(432_000_000).is_ok()
            && pll.set_post_divider(0, 2).is_ok()
        {
            pll.enable_clock(0);
            while !pll.clock_is_ready(0) {}
            Ok(())
        } else {
            Err(())
        }
    })
    .unwrap();
    if pllsetup.is_err() {
        todo!("Figure out what to do here\r\n");
    }

    fic.set_wait_states(6);

    let mut c = ctree.lock();
    c.main_mux_select(2); //use the pll as the sysclk
    drop(c);

    let ga = LockedArc::new(unsafe {
        crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 32 + 0, 0x4002_0000)
    });
    let gb = LockedArc::new(unsafe {
        crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 32 + 1, 0x4002_0400)
    });
    let gc = LockedArc::new(unsafe {
        crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 32 + 2, 0x4002_0800)
    });
    let gd = LockedArc::new(unsafe {
        crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 32 + 3, 0x4002_0c00)
    });
    let ge = LockedArc::new(unsafe {
        crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 32 + 4, 0x4002_1000)
    });
    let gf = LockedArc::new(unsafe {
        crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 32 + 5, 0x4002_1400)
    });
    let gg = LockedArc::new(unsafe {
        crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 32 + 6, 0x4002_1800)
    });
    let gh = LockedArc::new(unsafe {
        crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 32 + 7, 0x4002_1c00)
    });
    let gi = LockedArc::new(unsafe {
        crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 32 + 8, 0x4002_2000)
    });
    let gj = LockedArc::new(unsafe {
        crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 32 + 9, 0x4002_2400)
    });
    let gk = LockedArc::new(unsafe {
        crate::modules::gpio::stm32f769::Gpio::new(&ctree_provider, 32 + 10, 0x4002_2800)
    });

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

    //Setup the clocks of the usart to be the sysclock
    let mut r = rcc_mod.lock();
    for i in 0..7 {
        r.set_usart_mux(i, 1);
    }
    drop(r);

    {
        let mut gpio = crate::kernel::GPIO.lock();
        let mg = gpio.module(0);
        drop(gpio);
        let gpioa = mg.lock();
        let uart_tx = gpioa.get_pin(9);
        let uart_rx = gpioa.get_pin(10);
        drop(gpioa);

        let mut serials = crate::kernel::SERIAL.lock();
        serials.register_serial(
            LockedArc::new(unsafe {
                crate::modules::serial::stm32f769::Usart::new(
                    0x4001_1000,
                    ctree.get_ref(5 * 32 + 4),
                    [uart_tx, uart_rx],
                )
            })
            .into(),
        );
        serials.register_serial(
            LockedArc::new(unsafe {
                crate::modules::serial::stm32f769::Usart::new(
                    0x4000_4400,
                    ctree.get_ref(4 * 32 + 17),
                    [None, None],
                )
            })
            .into(),
        );
        serials.register_serial(
            LockedArc::new(unsafe {
                crate::modules::serial::stm32f769::Usart::new(
                    0x4000_4800,
                    ctree.get_ref(4 * 32 + 18),
                    [None, None],
                )
            })
            .into(),
        );
        serials.register_serial(
            LockedArc::new(unsafe {
                crate::modules::serial::stm32f769::Usart::new(
                    0x4000_4c00,
                    ctree.get_ref(4 * 32 + 19),
                    [None, None],
                )
            })
            .into(),
        );
        serials.register_serial(
            LockedArc::new(unsafe {
                crate::modules::serial::stm32f769::Usart::new(
                    0x4000_5000,
                    ctree.get_ref(4 * 32 + 20),
                    [None, None],
                )
            })
            .into(),
        );
        serials.register_serial(
            LockedArc::new(unsafe {
                crate::modules::serial::stm32f769::Usart::new(
                    0x4001_1400,
                    ctree.get_ref(5 * 32 + 5),
                    [None, None],
                )
            })
            .into(),
        );
        serials.register_serial(
            LockedArc::new(unsafe {
                crate::modules::serial::stm32f769::Usart::new(
                    0x4000_7800,
                    ctree.get_ref(4 * 32 + 30),
                    [None, None],
                )
            })
            .into(),
        );
        serials.register_serial(
            LockedArc::new(unsafe {
                crate::modules::serial::stm32f769::Usart::new(
                    0x4000_7c00,
                    ctree.get_ref(4 * 32 + 31),
                    [None, None],
                )
            })
            .into(),
        );
        drop(serials);
    }

    {
        let timer = unsafe {
            crate::modules::timer::stm32f769::TimerGroup::new(
                ctree.get_ref(5 * 32 + 0),
                0x4001_0000,
            )
        };
        let timer = crate::modules::timer::Timer::Stm32f769(LockedArc::new(timer));
        let mut timers = crate::kernel::TIMERS.lock();
        timers.register_timer(timer);
    }

    {
        use crate::modules::serial::SerialTrait;
        let mut serials = crate::kernel::SERIAL.lock();
        let serial = serials.module(0);
        drop(serials);
        let s = serial.lock();
        let serial_setup = s.setup(115200);
        drop(s);
        if serial_setup.is_ok() {
            let mut v = crate::VGA.lock();
            v.replace(TextDisplay::SerialDisplay(
                crate::modules::video::VideoOverSerial::new(serial),
            ));
            drop(v);
        }
    }

    let fmc_clock = ctree.get_ref(3 * 32);
    let mut fmc =
        unsafe { crate::modules::memory::stm32f769::SdramController::new(0xa000_0000, fmc_clock) };
    let sdram = fmc.setup_sdram();
    {
        let mut hm = super::HEAP_MANAGER.lock();
        hm.init(1, sdram.as_ptr() as usize, sdram.len());
    }
    
    let ltdc_clock = ctree_pll.get_pll_reference(2).unwrap();

    if ltdc_clock.set_vco_frequency(195_000_000).is_ok()
        && ltdc_clock.set_post_divider(2, 3).is_ok()
    {
        ltdc_clock.enable_clock(2);

        let mut gpio = crate::kernel::GPIO.lock();
        let mj = gpio.module(9);
        let mi = gpio.module(8);
        drop(gpio);
        let gpioi = mi.lock();
        let gpioj = mj.lock();

        let lcd_backlight = gpioi.get_pin(14).unwrap();
        let lcd_reset = gpioj.get_pin(15);

        drop(gpioi);
        drop(gpioj);
        drop(mi);
        drop(mj);

        let panel = Some(
            crate::modules::video::mipi_dsi::DsiPanel::OrisetechOtm8009a(LockedArc::new(
                crate::modules::video::mipi_dsi::OrisetechOtm8009a::new(
                    lcd_reset,
                    Some(lcd_backlight),
                ),
            )),
        );

        let dsi_config = crate::modules::video::mipi_dsi::MipiDsiConfig {
            link_speed: 500_000_000,
            num_lanes: 2,
            vcid: 0,
        };

        let dsi_byte_clock = ctree_pll.get_pll_reference(0).unwrap().get_ref(2);
        let dsi_clock1 =
            crate::modules::clock::ClockRef::Mux(ctree_pll.get_clock_mux(0).unwrap().clone());

        let dsi = unsafe {
            crate::modules::video::mipi_dsi::stm32f769::Module::new(
                &ctree_provider,
                [&dsi_byte_clock, &dsi_clock1],
                None,
                0x4001_6c00,
            )
        };

        if let Ok(display) = dsi.enable(&dsi_config, panel) {
            let mut displays = kernel::DISPLAYS.lock();
            displays.register_display(display);
        }
    }

    crate::main()
}

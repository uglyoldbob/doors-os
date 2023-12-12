//! Boot code for the stm32F769i-disco development board

use crate::modules::gpio::GpioTrait;

pub mod memory;

/// The entry point of the kernel
#[no_mangle]
pub extern "C" fn _start() -> ! {
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

    gj.set_output(13);
    gj.set_output(5);
    ga.set_output(12);
    loop {
        gj.write_output(13, true);
        gj.write_output(5, true);
        ga.write_output(12, true);
        gj.write_output(13, false);
        gj.write_output(5, false);
        ga.write_output(12, false);
    }

    crate::main()
}

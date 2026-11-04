//! Arm specific system code

use alloc::boxed::Box;

use crate::modules::interrupt::{arm, InterruptControllerTrait};

/// The system implementation for the cortex-m based stm32 boards
#[derive(Clone)]
pub struct Stm32System {}

impl Stm32System {
    /// Create a new system instance
    pub const fn new() -> Self {
        Self {}
    }
}

impl Default for Stm32System {
    fn default() -> Self {
        Self::new()
    }
}

impl super::SystemTrait for Stm32System {
    fn enable_interrupts(&self) {
        unsafe { core::arch::asm!("cpsie i", options(nomem, preserves_flags)) };
    }

    fn disable_interrupts(&self) {
        unsafe { core::arch::asm!("cpsid i", options(nomem, preserves_flags)) };
    }

    fn register_irq_handler<F: FnMut() + Send + Sync + crate::Interrupt + 'static>(
        &self,
        irq: u8,
        handler: F,
    ) -> bool {
        arm::register_handler(arm::EXCEPTION_BASE + irq as usize, Box::new(handler))
    }

    unsafe fn unregister_irq_handler(&self, irq: u8) {
        arm::unregister_handler(arm::EXCEPTION_BASE + irq as usize);
    }

    unsafe fn unregister_exception_handler(&self, exception: u8) {
        arm::unregister_handler(exception as usize);
    }

    fn register_exception_handler<F: FnMut() + Send + Sync + crate::Interrupt + 'static>(
        &self,
        exception: u8,
        handler: F,
    ) -> bool {
        arm::register_handler(exception as usize, Box::new(handler))
    }

    fn get_software_interrupt<F: FnMut() + Send + Sync + crate::Interrupt + 'static>(
        &self,
        _h: F,
    ) -> Option<super::SoftwareInterrupt> {
        // There is not currently a software interrupt implementation for arm
        None
    }

    fn enable_irq(&self, irq: u8) {
        if let Some(controller) = super::INTERRUPT_CONTROLLER.read().as_ref() {
            controller.enable_irq_sync(irq);
        }
    }

    fn disable_irq(&self, irq: u8) {
        if let Some(controller) = super::INTERRUPT_CONTROLLER.read().as_ref() {
            controller.disable_irq_sync(irq);
        }
    }

    fn init(&self) {
        // Nothing to initialize on the cortex-m system at this time
    }

    async fn acpi_debug(&self) {
        // There is no acpi on this system
    }

    fn main_stack(&self) -> (usize, usize) {
        // The main stack lives in the dedicated ram2 region defined by the linker script, the
        // stack starts at the top of the region and grows down.
        (0x2007_c000, 0x4000)
    }

    fn breakpoint(&self) -> Option<u8> {
        // The bkpt instruction is encoded with 0xbe as the first byte in thumb mode
        Some(0xbe)
    }

    fn create_process(&self, _b: &object::File) -> Result<(), ()> {
        // There is not currently a process implementation for arm
        Err(())
    }
}

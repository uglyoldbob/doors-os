//! This module is for code that directly handles interrupt mechanisms.

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod x86;

/// The trait implemented by interrupt controllers in the system
#[enum_dispatch::enum_dispatch]
pub trait InterruptControllerTrait {
    /// Indicate end of interrupt to the controller
    fn end_of_interrupt(&self, num: u8);
    /// enable the specified irq
    fn enable_irq(&self, num: u8);
    /// disable the specified irq
    fn disable_irq(&self, num: u8);
    /// Is the specified irq enabled?
    fn is_irq_enabled(&self, irq: u8) -> bool;
}

/// An interrupt controller for the system
#[enum_dispatch::enum_dispatch(InterruptControllerTrait)]
pub enum InterruptController {
    /// The x86 local apic
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    Apic(x86::LocalApic),
    /// the x86 pic
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    Pic(x86::Pic),
}

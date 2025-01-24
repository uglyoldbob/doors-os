//! Code for the pci bus

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod x86;

/// The pci bus trait
#[enum_dispatch::enum_dispatch]
pub trait PciBusTrait {
    /// Setup the bus
    fn setup(&self);
}

/// a pci bus instance
#[enum_dispatch::enum_dispatch(PciBusTrait)]
pub enum PciBus {
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    /// X86 pci bus
    X86Pci(x86::PciBus),
}

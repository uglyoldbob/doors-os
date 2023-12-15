//! Clock providers for the stm32f769i-disco board

use alloc::sync::Arc;

/// The external oscillator for the stm32f769
pub struct ExternalOscillator {
    /// The hardware for configuring the oscillator
    cc: Arc<crate::modules::clock::ClockProvider>,
}

impl ExternalOscillator {
    /// Create a new external oscillator with the specified frequency
    pub unsafe fn new(frequency: u32, rcc: &Arc<crate::modules::clock::ClockProvider>) -> Self {
        Self { cc: rcc.clone() }
    }
}

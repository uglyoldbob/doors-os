//! Drivers for memory based hardware. This includes sdram controllers, sram controllers, and nand flash mapped directly to memory.

#[cfg(kernel_machine = "stm32f769i-disco")]
pub mod stm32f769;

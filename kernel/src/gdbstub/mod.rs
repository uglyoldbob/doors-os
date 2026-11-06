//! Code for operating the gdbstub
//!
//! The [`x86`] submodule holds the register layouts shared with the rest of the
//! kernel (the scheduler uses them to read and write the saved register state of
//! a task), so it is compiled in for every x86 machine whether or not the
//! gdbstub is enabled. The stub itself is only built when the kernel
//! configuration enables it.

#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
pub mod x86;

/// The gdbstub target and the loops that service a connected debugger
#[doors_macros::config_check_equals_attr(gdbstub, "true")]
mod runner;

/// The entry points for running the gdbstub, if it is enabled in the kernel
/// configuration
#[doors_macros::config_check_equals_attr(gdbstub, "true")]
pub use runner::{run, sync_run};

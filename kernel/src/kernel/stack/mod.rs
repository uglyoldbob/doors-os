//! Contains code for unwinding the stack when there is a panic in the kernel
//!
//! This module provides DWARF-based stack unwinding capabilities using the gimli crate
//! to parse debug information and provide meaningful stack traces.

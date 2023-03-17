#![no_std]
#![deny(missing_docs)]

//! This crate defines the traits for in used by the various parts of the kernel

/// The video module is for all things directly related to video output.
pub mod video;

/// A fixed string type that allows for strings of up to 32 characters.
pub type FixedString = arraystring::ArrayString<arraystring::typenum::U32>;
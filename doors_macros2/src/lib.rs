//! This crate defines various macros used in the Doors kernel.

#![no_std]
#![deny(missing_docs)]

/// This macro re-exports definitions required to fill out enum variants
#[macro_export]
macro_rules! enum_reexport {
    ( $m:ident) => {
        /// A module for re-exporting things for enumeration fillout
        pub mod doors_enum_variants {
            pub use super::$m::doors_enum_variants::*;
        }
    };
}

/// A macro for formatting strings of a maximum length for printing from the kernel
#[macro_export]
macro_rules! fixed_string_format {
    ( $($arg:tt)* ) => {
        {
            let mut a: crate::FixedString = crate::FixedString::new();
            let r = core::fmt::write(
                &mut a,
                core::format_args!($($arg)*),
            );
            match r {
                core::result::Result::Ok(_) => a,
                core::result::Result::Err(_) => {
                    let mut b = crate::FixedString::new();
                    b.push_str("Error parsing string\r\n");
                    b
                }
            }
        }
    };
}

//! This crate defines various macros used in the Doors kernel.

#![no_std]
#![deny(missing_docs)]

extern crate alloc;

#[cfg(feature = "backtrace")]
pub mod backtrace;

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

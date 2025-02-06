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

/// A macro for printing strings from the kernel
#[macro_export]
macro_rules! kernel_print {
    ( $($arg:tt)* ) => {
        {
            let mut a: crate::FixedString = crate::FixedString::new();
            let r = core::fmt::write(
                &mut a,
                core::format_args!($($arg)*),
            );
            let mut v = crate::VGA.lock();
            let mut vga = v.as_mut();
            if let core::option::Option::Some(vga) = vga {
                match r {
                    core::result::Result::Ok(_) => vga.print_str(a.as_str()),
                    core::result::Result::Err(_) => vga.print_str("Error parsing string\r\n"),
                }
            }
        }
    };
}

/// Like kernel print, but it might allocate memory
#[macro_export]
macro_rules! kernel_print_alloc {
    ( $($arg:tt)* ) => {
        {
            let mut a: alloc::string::String = alloc::string::String::new();
            let r = core::fmt::write(
                &mut a,
                core::format_args!($($arg)*),
            );
            let mut v = crate::VGA.lock();
            let mut vga = v.as_mut();
            if let core::option::Option::Some(vga) = vga {
                match r {
                    core::result::Result::Ok(_) => vga.print_str(a.as_str()),
                    core::result::Result::Err(_) => vga.print_str("Error parsing string\r\n"),
                }
            }
        }
    };
}

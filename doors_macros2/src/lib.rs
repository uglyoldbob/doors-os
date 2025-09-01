//! This crate defines various macros used in the Doors kernel.

#![no_std]
#![deny(missing_docs)]

extern crate alloc;

pub mod backtrace;

/// This macro re-exports definitions required to fill out enum variants
#[macro_export]
macro_rules! enum_reexport {
    ( $o:ident, $m:ident ) => {
        /// A module for exporting things for enumeration fillout
        #[allow(non_snake_case)]
        pub mod $o {
            pub use super::super::$m::doors_enum_variants::$o::*;
        }
    };
    ( $o:ident, $($m:ident),+ ) => {
        /// A module for exporting things for enumeration fillout
        #[allow(non_snake_case)]
        pub mod $o {
            $(pub use super::super::$m::doors_enum_variants::$o::*;)+
        }
    };
}

/// This macro exports multiple definitions required to fill out enum variants
#[macro_export]
macro_rules! enum_export_builder {
    ( $($m2:item)+ ) => {
        /// A module for exporting things for enumeration fillout
        pub mod doors_enum_variants {
            $($m2)+
        }
    };
}

/// This macro exports definitions required to fill out enum variants
#[macro_export]
macro_rules! enum_export {
    ( $m:ident, $($m2:ident),+ ) => {
        /// A module for exporting things for enumeration fillout
        #[allow(non_snake_case)]
        pub mod $m {
            $(pub use super::super::$m2;)+
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

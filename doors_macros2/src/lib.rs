#![no_std]
#![deny(missing_docs)]

//! This crate defines various macros used in the Doors kernel.

/// A macro for printing strings from the kernel
#[macro_export]
macro_rules! kernel_print {
    ( $($arg:tt)* ) => {
        {
            let mut a: FixedString = FixedString::new();
            match core::fmt::write(
                &mut a,
                format_args!($($arg)*),
            ) {
                Ok(_) => VGA.lock().print_str(a.as_str()),
                Err(_) => VGA.lock().print_str("Error parsing string\r\n"),
            }
        }
    };
}

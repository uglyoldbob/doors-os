#![no_std]
#![deny(missing_docs)]

//! This crate defines various macros used in the Doors kernel.

/// A macro for printing strings from the kernel
#[macro_export]
macro_rules! kernel_print {
    ( $($arg:tt)* ) => {
        {
            let mut a: FixedString = FixedString::new();
            let mut v = VGA.lock();
            let mut vga = v.as_mut();
            if let Some(vga) = vga {
                match core::fmt::write(
                    &mut a,
                    format_args!($($arg)*),
                ) {
                    Ok(_) => vga.print_str(a.as_str()),
                    Err(_) => vga.print_str("Error parsing string\r\n"),
                }
            }
        }
    };
}

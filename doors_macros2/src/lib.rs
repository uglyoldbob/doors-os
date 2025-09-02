//! This crate defines various macros used in the Doors kernel.

#![no_std]
#![deny(missing_docs)]

extern crate alloc;

#[cfg(feature = "backtrace")]
pub mod location;
#[cfg(feature = "backtrace")]
mod frame;
#[cfg(feature = "backtrace")]
mod framed;
#[cfg(feature = "backtrace")]
mod linked_list;

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

/// Produces a [`Location`] when invoked in a function body.
///
/// ```
/// use async_backtrace::{location, Location};
///
/// #[tokio::main]
/// async fn main() {
///     assert_eq!(location!().to_string(), "rust_out::main::{{closure}} at backtrace/src/location.rs:8:16");
///
///     async {
///         assert_eq!(location!().to_string(), "rust_out::main::{{closure}}::{{closure}} at backtrace/src/location.rs:11:20");
///     }.await;
///     
///     (|| async {
///         assert_eq!(location!().to_string(), "rust_out::main::{{closure}}::{{closure}}::{{closure}} at backtrace/src/location.rs:15:20");
///     })().await;
/// }
/// ```
#[macro_export]
macro_rules! location {
    () => {{
        macro_rules! fn_name {
            () => {{
                fn type_name_of_val<T: ?Sized>(_: &T) -> &'static str {
                    core::any::type_name::<T>()
                }
                type_name_of_val(&|| {})
                    .strip_suffix("::{{closure}}")
                    .unwrap()
            }};
        }
        $crate::location::Location::from_components(fn_name!(), &(file!(), line!(), column!()))
    }};
}

/// Include the annotated async expression in backtraces and taskdumps.
///
/// This, for instance:
/// ```
/// # #[tokio::main] async fn main() {
/// # async fn foo() {}
/// # async fn bar() {}
/// tokio::spawn(async_backtrace::frame!(async {
///     foo().await;
///     bar().await;
/// })).await;
/// # }
/// ```
/// ...expands, roughly, to:
/// ```
/// # #[tokio::main] async fn main() {
/// # async fn foo() {}
/// # async fn bar() {}
/// tokio::spawn(async_backtrace::location!().frame(async {
///     foo().await;
///     bar().await;
/// })).await;
/// # }
/// ```
#[macro_export]
macro_rules! frame {
    ($async_expr:expr) => {
        $crate::location!().frame($async_expr)
    };
}

/// Do some defer stuff
pub(crate) fn defer<F: FnOnce() -> R, R>(f: F) -> impl Drop {
    struct Defer<F: FnOnce() -> R, R>(Option<F>);

    impl<F: FnOnce() -> R, R> Drop for Defer<F, R> {
        fn drop(&mut self) {
            self.0.take().unwrap()();
        }
    }

    Defer(Some(f))
}
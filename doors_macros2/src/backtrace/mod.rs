//! Backtrace code and macros

pub mod location;

/// Instruments an async function
#[macro_export]
macro_rules! frame {
    ($async_expr:expr) => {{
        let l = $crate::location!();
        crate::executor::register_location(l).await;
        let r = $async_expr.await;
        crate::executor::unregister_location(l).await;
        r
    }};
}
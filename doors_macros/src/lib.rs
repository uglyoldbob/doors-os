#![deny(missing_docs)]

//! This crate defines various macros used in the Doors kernel.

use std::sync::Mutex;

use quote::quote;
use syn::parse_macro_input;

lazy_static::lazy_static! {
    /// The number of test functions in the kernel
    static ref TEST_CALL_QUANTITY: Mutex<Option<usize>> = Mutex::new(None);
}

/// Defines the required doors test structure
#[proc_macro]
pub fn use_doors_test(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    quote! {
        /// The struct for test functions
        pub struct DoorsTester {}
    }
    .into()
}

#[proc_macro_attribute]
/// This attribute marks a function as a specific function that runs a test
pub fn doors_test(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    assert!(attr.is_empty());
    let item2 = item.clone();
    let f = parse_macro_input!(item2 as syn::ItemFn);
    let resitem = quote! {fn test_function() -> Result<(),()> { Err(()) }}.into();
    let fcmp = parse_macro_input!(resitem as syn::ItemFn);
    if fcmp.sig.output != f.sig.output {
        panic!("Function {} must return a Result<(),()>", f.sig.ident);
    }
    let index = {
        let mut test_calls = TEST_CALL_QUANTITY.lock().unwrap();
        match &mut *test_calls {
            None => {
                *test_calls = Some(1);
                0
            }
            Some(t) => {
                let oldval = *t;
                *t += 1;
                oldval
            }
        }
    };
    let fcall = f.sig.ident;
    let fcall2 = fcall.to_string();
    let item: proc_macro2::TokenStream = item.into();
    let id = quote::format_ident!("test_{}", index);
    let q = quote! {
        #item
        impl crate::DoorsTester {
            /// Test function #index
            pub fn #id() -> Result<(),()> {
                let r = #fcall();
                if r.is_err() {
                    doors_macros2::kernel_print!("Test {} failed\r\n", #fcall2);
                }
                r
            }
        }
    };
    q.into()
}

/// This creates the function that runs all of the tests
#[proc_macro]
pub fn define_doors_test_runner(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut testa = TEST_CALL_QUANTITY.lock().unwrap();
    let testa = testa.take().unwrap();

    let i = 0..testa;
    let calls = i.into_iter().map(|i| {
        let ident = quote::format_ident!("test_{}", i);
        quote!(Self::#ident)
    });

    quote! {
        impl DoorsTester {
            fn doors_test_main() -> Result<(),()> {
                #(#calls()?;)*
                Ok(())
            }
        }
    }
    .into()
}

/// This macro creates an 32-bit interrupt function, with the appopriate entry and exit code
#[proc_macro_attribute]
pub fn interrupt(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let fun: syn::ItemFn = syn::parse(item).unwrap();
    let fname = fun.sig.ident.clone();
    let asmname = syn::Ident::new(
        (fname.to_string() + "_asm").as_str(),
        proc_macro2::Span::mixed_site(),
    );
    let assembly = format!(
        "
    .section .text
    .global {0}
    .code32
    .extern {1}
    {0}:
        push eax
        call {1}
        pop eax
        iret",
        asmname, fname
    );
    quote! {
        extern {
            /// The assembly code for an interrupt function
            pub fn #asmname ();
        }
        core::arch::global_asm!(#assembly);
        #[no_mangle]
        #fun
    }
    .into()
}

/// This macro creates a 64-bit interrupt function, with the appopriate entry and exit code
#[proc_macro_attribute]
pub fn interrupt_64(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let fun: syn::ItemFn = syn::parse(item).unwrap();
    let fname = fun.sig.ident.clone();
    let asmname = syn::Ident::new(
        (fname.to_string() + "_asm").as_str(),
        proc_macro2::Span::mixed_site(),
    );
    let assembly = format!(
        "
    .section .text
    .global {0}
    .code64
    .extern {1}
    {0}:
        push rax
        call {1}
        pop rax
        iret",
        asmname, fname
    );
    quote! {
        extern {
            /// The assembly code for an interrupt function
            pub fn #asmname ();
        }
        core::arch::global_asm!(#assembly);
        #[no_mangle]
        #fun
    }
    .into()
}

/// This macro creates a 64-bit interrupt function taking a single argument, with the appopriate entry and exit code
#[proc_macro_attribute]
pub fn interrupt_arg_64(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let fun: syn::ItemFn = syn::parse(item).unwrap();
    let fname = fun.sig.ident.clone();
    let asmname = syn::Ident::new(
        (fname.to_string() + "_asm").as_str(),
        proc_macro2::Span::mixed_site(),
    );
    let assembly = format!(
        "
    .section .text
    .global {0}
    .code64
    .extern {1}
    {0}:
        pop rdi
        push rax
        call {1}
        pop rax
        iret",
        asmname, fname
    );
    quote! {
        extern {
            /// The assembly code for an interrupt function
            pub fn #asmname ();
        }
        core::arch::global_asm!(#assembly);
        #[no_mangle]
        #fun
    }
    .into()
}

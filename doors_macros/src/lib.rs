#![deny(missing_docs)]

//! This crate defines various macros used in the Doors kernel.

use proc_macro::TokenStream;
use quote::quote;

/// This macro creates an 32-bit interrupt function, with the appopriate entry and exit code
#[proc_macro_attribute]
pub fn interrupt(_attr: TokenStream, item: TokenStream) -> TokenStream {
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
pub fn interrupt_64(_attr: TokenStream, item: TokenStream) -> TokenStream {
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
pub fn interrupt_arg_64(_attr: TokenStream, item: TokenStream) -> TokenStream {
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

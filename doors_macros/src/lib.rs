#![deny(missing_docs)]

//! This crate defines various macros used in the Doors kernel.

use std::{
    collections::{BTreeMap, HashSet},
    io::Read,
    str::FromStr,
    sync::Mutex,
};

use quote::quote;
use syn::parse_macro_input;

mod config;
use config::KernelConfig;

#[derive(Debug)]
struct EnumData {
    variants: Vec<String>,
    variant_names: HashSet<String>,
}

lazy_static::lazy_static! {
    /// The number of test functions in the kernel
    static ref TEST_CALL_QUANTITY: Mutex<Option<usize>> = Mutex::new(None);
    /// The enum builder data
    static ref ENUM_BUILDER: Mutex<BTreeMap<String, EnumData>> = Mutex::new(BTreeMap::new());
    /// The kernel config
    static ref KERNEL_CONFIG: Mutex<Option<KernelConfig>> = Mutex::new(None);
}

/// Define the kernel config for the kernel build script
#[proc_macro]
pub fn define_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    assert!(input.is_empty());
    let c = include_str!("config.rs");
    let ts = proc_macro2::TokenStream::from_str(c).unwrap();
    quote!(
        mod config {
            #ts
        }
    )
    .into()
}

/// Load the kernel config for building the kernel
#[proc_macro]
pub fn load_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    assert!(input.is_empty());
    let mdir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut p = std::path::PathBuf::from_str(&mdir).unwrap();
    p.push("config.toml");
    let mut config = std::fs::File::open(p).expect("Failed to open kernel configuration");
    let mut config_contents = Vec::new();
    config
        .read_to_end(&mut config_contents)
        .expect("Failed to read kernel configuration");
    let config =
        String::from_utf8(config_contents).expect("Invalid contents in kernel configuration");
    let config = toml::from_str::<KernelConfig>(&config).expect("Invalid kernel configuration");
    let mut m = KERNEL_CONFIG.lock().unwrap();
    if m.is_some() {
        panic!("Kernel config already loaded");
    }
    m.replace(config);
    quote!().into()
}

struct ConfigCheckBlock {
    ident: syn::Ident,
    block: syn::Block,
}

impl syn::parse::Parse for ConfigCheckBlock {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = syn::Ident::parse(input)?;
        syn::token::Comma::parse(input)?;
        let block = syn::Block::parse(input)?;
        let s = Self { ident, block };
        Ok(s)
    }
}

/// Retrieve a boolean value from the kernel config and use it to enable a block of code
#[proc_macro]
pub fn config_check_bool(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let f = parse_macro_input!(input as ConfigCheckBlock);
    let m = KERNEL_CONFIG.lock().unwrap();
    let m = m.as_ref().unwrap();
    let val = m.get_field(&f.ident.to_string());
    let block = f.block;
    if val {
        quote!(#block).into()
    } else {
        quote!().into()
    }
}

/// Retrieve a boolean value from the kernel config
#[proc_macro]
pub fn config_build_struct(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let m = KERNEL_CONFIG.lock().unwrap();
    let m = m.as_ref().unwrap();
    let item2 = item.clone();
    let mut f = parse_macro_input!(item2 as syn::ExprStruct);

    let mod_field = |mut elem: syn::FieldValue| {
        let field_use = elem.attrs.iter().find_map(|attr| {
            if let Some(a) = attr.path.get_ident() {
                if *a == "doorsconfig" {
                    let p = attr.parse_meta().unwrap();
                    if let syn::Meta::NameValue(n) = p {
                        if let syn::Lit::Str(l) = n.lit {
                            let name = l.value();
                            let val: bool = m.get_field(&name);
                            Some(val)
                        } else {
                            panic!("Expected a string literal");
                        }
                    } else {
                        panic!("Expected the form doorsconfig = \"something\"");
                    }
                } else {
                    None
                }
            } else {
                None
            }
        });
        let t = elem
            .attrs
            .clone()
            .into_iter()
            .filter(|attr| {
                if let Some(a) = attr.path.get_ident() {
                    *a != "doorsconfig"
                } else {
                    true
                }
            })
            .collect();
        elem.attrs = t;
        if let Some(u) = field_use {
            if u {
                Some(elem.to_owned())
            } else {
                None
            }
        } else {
            Some(elem.to_owned())
        }
    };

    let mut punc: syn::punctuated::Punctuated<syn::FieldValue, syn::token::Comma> =
        syn::punctuated::Punctuated::new();
    for field in f.fields.clone().into_iter().filter_map(mod_field) {
        punc.push_value(field);
        punc.push_punct(syn::token::Comma::default());
    }
    f.fields = punc;
    quote!(#f).into()
}

/// Check a boolean value from the kernel config to enable code
#[proc_macro_attribute]
pub fn config_check_struct(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    assert!(attr.is_empty());
    let m = KERNEL_CONFIG.lock().unwrap();
    let m = m.as_ref().unwrap();
    let item2 = item.clone();
    let mut f = parse_macro_input!(item2 as syn::ItemStruct);

    let mod_field = |mut elem: syn::Field| {
        let field_use = elem.attrs.iter().find_map(|attr| {
            if let Some(a) = attr.path.get_ident() {
                if *a == "doorsconfig" {
                    let p = attr.parse_meta().unwrap();
                    if let syn::Meta::NameValue(n) = p {
                        if let syn::Lit::Str(l) = n.lit {
                            let name = l.value();
                            let val: bool = m.get_field(&name);
                            Some(val)
                        } else {
                            panic!("Expected a string literal");
                        }
                    } else {
                        panic!("Expected the form doorsconfig = \"something\"");
                    }
                } else {
                    None
                }
            } else {
                None
            }
        });
        let t = elem
            .attrs
            .clone()
            .into_iter()
            .filter(|attr| {
                if let Some(a) = attr.path.get_ident() {
                    *a != "doorsconfig"
                } else {
                    true
                }
            })
            .collect();
        elem.attrs = t;
        if let Some(u) = field_use {
            if u {
                Some(elem.to_owned())
            } else {
                None
            }
        } else {
            Some(elem.to_owned())
        }
    };

    f.fields = match f.fields {
        syn::Fields::Unit => syn::Fields::Unit,
        syn::Fields::Named(mut n) => {
            let mut punc: syn::punctuated::Punctuated<syn::Field, syn::token::Comma> =
                syn::punctuated::Punctuated::new();
            for field in n.named.clone().into_iter().filter_map(mod_field) {
                punc.push_value(field);
                punc.push_punct(syn::token::Comma::default());
            }
            n.named = punc;
            syn::Fields::Named(n)
        }
        syn::Fields::Unnamed(mut n) => {
            let mut punc: syn::punctuated::Punctuated<syn::Field, syn::token::Comma> =
                syn::punctuated::Punctuated::new();
            for field in n.unnamed.clone().into_iter().filter_map(mod_field) {
                punc.push_value(field);
                punc.push_punct(syn::token::Comma::default());
            }
            n.unnamed = punc;
            syn::Fields::Unnamed(n)
        }
    };
    quote!(#f).into()
}

/// A macro that declares that an enum will be created
#[proc_macro]
pub fn declare_enum(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let f = parse_macro_input!(input as syn::Ident);
    let mut e = ENUM_BUILDER.lock().unwrap();
    let n = f.to_string();
    let n2 = n.clone();
    if let std::collections::btree_map::Entry::Vacant(e) = e.entry(n) {
        e.insert(EnumData {
            variants: Vec::new(),
            variant_names: HashSet::new(),
        });
    } else {
        panic!("Enum {} was already declared", n2);
    }
    quote!().into()
}

/// A macro that adds a variant to an enum
#[proc_macro_attribute]
pub fn enum_variant(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let f = parse_macro_input!(attr as syn::Ident);
    let item2 = item.clone();
    let i = parse_macro_input!(item2 as syn::ItemStruct);
    let mut e = ENUM_BUILDER.lock().unwrap();
    let entry = e.get_mut(&f.to_string()).unwrap();
    let index = entry.variants.len();
    let varname = i.ident;
    let comments = i.attrs;
    let newid = if entry.variant_names.contains(&varname.to_string()) {
        quote::format_ident!("{}{}", varname, index)
    } else {
        quote::format_ident!("{}", varname)
    };
    let q = quote! {
        #(#comments)*
        #newid(doors_enum_variants::#varname)
    };
    entry.variants.push(q.to_string());
    let item: proc_macro2::TokenStream = item.into();
    quote! {
        #item
        /// A module for making variants accessible
        pub mod doors_enum_variants {
            pub use super::#varname;
        }
    }
    .into()
}

/// A macro that adds the previously defined variants into the enum, adding an enum_dispatch for a given trait
#[proc_macro_attribute]
pub fn fill_enum_with_variants(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let dispatch = parse_macro_input!(attr as syn::Ident);
    let mut f = parse_macro_input!(item as syn::ItemEnum);
    let name = f.ident.clone();
    let vars = &mut f.variants;

    let mut e = ENUM_BUILDER.lock().unwrap();
    let n = name.to_string();
    let data = e.remove(&n).unwrap();
    if data.variants.is_empty() {
        panic!("No variants defined for {}", n);
    }
    for d in data.variants {
        let ts = proc_macro::TokenStream::from_str(&d).unwrap();
        let v = parse_macro_input!(ts as syn::Variant);
        vars.push(v);
    }
    let fts = quote::ToTokens::into_token_stream(f);
    quote! {
        #[enum_dispatch::enum_dispatch(#dispatch)]
        #fts
    }
    .into()
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

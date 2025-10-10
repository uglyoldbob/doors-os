#![deny(missing_docs)]
#![cfg_attr(feature = "todo", feature(proc_macro_span))]

//! This crate defines various macros used in the Doors kernel.

use std::{
    collections::{BTreeMap, HashSet},
    io::Read,
    str::FromStr,
    sync::Mutex,
};

#[cfg(feature = "backtrace")]
mod backtrace;

use quote::quote;
use syn::parse_macro_input;

mod config;
use config::KernelConfig;

#[derive(Debug)]
struct EnumData {
    variants: Vec<String>,
    variant_names: HashSet<String>,
}

struct TodoList {
    items: Vec<String>,
}

impl TodoList {
    const fn new() -> Self {
        Self { items: Vec::new() }
    }
}

/// The todo list for the kernel
static TODOLIST: Mutex<Option<TodoList>> = Mutex::new(Some(TodoList::new()));

/// The number of test functions in the kernel
static TEST_CALL_QUANTITY: Mutex<Option<usize>> = Mutex::new(None);
/// The kernel config
static KERNEL_CONFIG: Mutex<Option<KernelConfig>> = Mutex::new(None);

/// Insert a todo list entry into the todolist and do nothing else
#[cfg(feature = "todo")]
#[proc_macro]
pub fn todo_item(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item2 = item.clone();
    let f = parse_macro_input!(item2 as syn::LitStr);
    let mut list = TODOLIST.lock().expect("Unable to lock todolist");
    let list = list.as_mut();
    if let Some(list) = list {
        let ds = proc_macro::Span::call_site();
        list.items.push(format!(
            "{} @ {:?} line {}",
            f.value(),
            ds.file(),
            ds.start().line()
        ));
    }
    quote!().into()
}

/// Insert a todo list entry into the todolist and also emit a todo macro
#[cfg(feature = "todo")]
#[proc_macro]
pub fn todo(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item2 = item.clone();
    let f = parse_macro_input!(item2 as Option<syn::LitStr>).unwrap_or(syn::LitStr::new(
        "TODO",
        proc_macro::Span::call_site().into(),
    ));
    let mut list = TODOLIST.lock().expect("Unable to lock todo list");
    let list = list.as_mut();
    if let Some(list) = list {
        let ds = proc_macro::Span::call_site();
        list.items.push(format!(
            "{} @ {:?} line {}",
            f.value(),
            ds.file(),
            ds.start().line()
        ));
    }
    quote!(todo!(#f)).into()
}

/// Insert a todo list entry into the todolist and also panic
#[cfg(feature = "todo")]
#[proc_macro]
pub fn todo_item_panic(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item2 = item.clone();
    let f = parse_macro_input!(item2 as syn::LitStr);
    let mut list = TODOLIST.lock().expect("Unable to lock todo list");
    let list = list.as_mut();
    if let Some(list) = list {
        let ds = proc_macro::Span::call_site();
        list.items.push(format!(
            "{} @ {:?} line {}",
            f.value(),
            ds.file(),
            ds.start().line()
        ));
    }
    quote!(
        panic!(#f);
    )
    .into()
}

/// Populate the todo list for the kernel
#[proc_macro]
pub fn populate_todo_list(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    assert!(input.is_empty());
    let list = TODOLIST.lock().expect("Unable to lock todo list").take();
    if let Some(list) = list {
        let things = list.items.iter().map(|i| {
            let msg = format!("* {}", i);
            quote! {
                #[doc = #msg]
            }
        });
        quote!(
            /// The todo list. This is a list of things that need to be done.
            #(#things)*
            struct TodoList {}
        )
        .into()
    } else {
        quote!().into()
    }
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
    let check = {
        let mut m = KERNEL_CONFIG.lock().unwrap();
        if m.is_some() {
            Err("Kernel config already loaded")
        } else {
            m.replace(config);
            Ok(())
        }
    };
    if let Err(e) = check {
        panic!("{}", e);
    }

    quote!().into()
}

struct ConfigCheckValue {
    ident: syn::Ident,
    val: syn::LitStr,
}

impl syn::parse::Parse for ConfigCheckValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = syn::Ident::parse(input)?;
        syn::token::Comma::parse(input)?;
        let block = syn::Lit::parse(input)?;
        Ok(if let syn::Lit::Str(val) = block {
            Self { ident, val }
        } else {
            panic!("Expected a string literal for argument 2");
        })
    }
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

struct ConfigCheckBlock2 {
    ident: syn::Ident,
    block: syn::Expr,
}

impl syn::parse::Parse for ConfigCheckBlock2 {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = syn::Ident::parse(input)?;
        syn::token::Comma::parse(input)?;
        let block = syn::Expr::parse(input)?;
        let s = Self { ident, block };
        Ok(s)
    }
}

/// Check a boolean value from the kernel config to enable code
#[proc_macro_attribute]
pub fn config_check(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let f = parse_macro_input!(attr as ConfigCheckValue);
    let go = {
        let m = KERNEL_CONFIG.lock().unwrap();
        m.as_ref()
            .map(|a| a.check_field(&f.ident.to_string(), &f.val.value()))
    }
    .unwrap();
    if go {
        let item: proc_macro2::TokenStream = item.into();
        quote!(#item).into()
    } else {
        quote!().into()
    }
}

/// Compare a value from the kernel config to a specified string, and return the result
#[proc_macro]
pub fn config_check_equals(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let f = parse_macro_input!(input as ConfigCheckValue);
    let check = {
        let m = KERNEL_CONFIG.lock().unwrap();
        m.as_ref()
            .map(|a| a.check_field(&f.ident.to_string(), &f.val.value()))
    };
    let val = check.unwrap();
    if val {
        quote!(true).into()
    } else {
        quote!(false).into()
    }
}

/// Conditionally enable an item with an equals comparision from the kernel config
#[proc_macro_attribute]
pub fn config_check_equals_attr(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let f = parse_macro_input!(attr as ConfigCheckValue);
    let check = {
        let m = KERNEL_CONFIG.lock().unwrap();
        m.as_ref()
            .map(|a| a.check_field(&f.ident.to_string(), &f.val.value()))
    };
    let val = check.unwrap();
    if val {
        let item: proc_macro2::TokenStream = item.into();
        quote!(#item).into()
    } else {
        quote!().into()
    }
}

/// Conditionally enable modules in an enum based on the value of the entry in the modules variable
#[proc_macro_attribute]
pub fn enum_module_filter(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item2 = item.clone();
    let m = KERNEL_CONFIG.lock().unwrap();
    let m = m.as_ref().unwrap();
    let mut f = parse_macro_input!(item2 as syn::ItemEnum);
    let mut new_variants = syn::punctuated::Punctuated::new();
    for v in &mut f.variants {
        let name = &v.ident;
        let mut include_me = false;
        let mut found_attr = false;
        for a in &v.attrs {
            let p = &a.meta;
            let ca = if let syn::Meta::NameValue(n) = p {
                let mut found_module_filt = false;
                for s in &n.path.segments {
                    if s.ident == "doors_module" {
                        found_module_filt = true;
                    }
                }
                if found_module_filt {
                    if let syn::Expr::Lit(l) = &n.value {
                        if let syn::Lit::Str(l) = &l.lit {
                            Some(l.value())
                        } else {
                            panic!("Expected a string literal");
                        }
                    } else {
                        panic!("Expected a string literal");
                    }
                } else {
                    None
                }
            } else {
                panic!("Expected the form doors_module = \"something\"");
            };
            if let Some(ca) = ca {
                found_attr = true;
                if m.modules.contains(&ca) {
                    include_me = true;
                }
            }
        }
        let t = v
            .attrs
            .clone()
            .into_iter()
            .filter(|attr| {
                if let Some(a) = attr.path().get_ident() {
                    *a != "doors_module"
                } else {
                    true
                }
            })
            .collect();
        v.attrs = t;
        if !found_attr || include_me {
            if !new_variants.is_empty() {
                new_variants.push_punct(syn::token::Comma::default());
            }
            new_variants.push(v.clone());
        }
    }
    f.variants = new_variants;
    quote!(#f).into()
}

/// Conditionally enable an item with an equals comparision from the kernel config for a module
#[proc_macro_attribute]
pub fn module_builtin_attr(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let f = parse_macro_input!(attr as ConfigCheckValue);
    let check = {
        let m = KERNEL_CONFIG.lock().unwrap();
        let cv = f.val.value();
        let mv = match cv.as_str() {
            "false" => false,
            "true" => true,
            _ => panic!("Invalid value {}", cv),
        };
        m.as_ref()
            .map(|a| mv == a.modules.contains(&f.ident.to_string()))
    };
    let val = check.unwrap();

    if val {
        let item: proc_macro2::TokenStream = item.into();
        quote!(#item).into()
    } else {
        // Remove the module declaration entirely
        quote!().into()
    }
}

/// Check modules to compile from the kernel config and use it to enable a block of code
#[proc_macro]
pub fn module_builtin(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let f = parse_macro_input!(input as ConfigCheckBlock2);
    print!("CHECKING Include the block for {}", f.ident);
    let check = {
        let m = KERNEL_CONFIG.lock().unwrap();
        m.as_ref().map(|a| a.modules.contains(&f.ident.to_string()))
    };
    let val = check.unwrap();
    let block = f.block;
    if val {
        print!("Including the block for {}", f.ident);
        quote!(#block).into()
    } else {
        print!("NOT Including the block for {}", f.ident);
        quote!().into()
    }
}

/// Retrieve a boolean value from the kernel config and use it to enable a block of code
#[proc_macro]
pub fn config_check_bool(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let f = parse_macro_input!(input as ConfigCheckBlock);
    let check = {
        let m = KERNEL_CONFIG.lock().unwrap();
        m.as_ref()
            .map(|a| a.check_field(&f.ident.to_string(), "true"))
    };
    let val = check.unwrap();
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
    let m = {
        let m = KERNEL_CONFIG.lock().unwrap();
        m.as_ref().map(|a| a.to_owned())
    }
    .unwrap();
    let item2 = item.clone();
    let mut f = parse_macro_input!(item2 as syn::ExprStruct);

    let mod_field = |mut elem: syn::FieldValue| {
        let field_use = elem.attrs.iter().find_map(|attr| {
            if let Some(a) = attr.path().get_ident() {
                if *a == "doorsconfig" {
                    let p = &attr.meta;
                    if let syn::Meta::NameValue(n) = p {
                        if let syn::Expr::Lit(l) = &n.value {
                            if let syn::Lit::Str(l) = &l.lit {
                                let name = l.value();
                                let val: bool = m.check_field(&name, "true");
                                Some(val)
                            } else {
                                panic!("Expected a string literal");
                            }
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
                if let Some(a) = attr.path().get_ident() {
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

/// Check a boolean value from the kernel config to conditionally disable items in a structure
#[proc_macro_attribute]
pub fn config_check_struct(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    assert!(attr.is_empty());
    let m = {
        let m = KERNEL_CONFIG.lock().unwrap();
        m.as_ref().map(|a| a.to_owned())
    }
    .unwrap();
    let item2 = item.clone();
    let mut f = parse_macro_input!(item2 as syn::ItemStruct);

    let mod_field = |mut elem: syn::Field| {
        let field_use = elem.attrs.iter().find_map(|attr| {
            if let Some(a) = attr.path().get_ident() {
                if *a == "doorsconfig" {
                    let p = &attr.meta;
                    if let syn::Meta::NameValue(n) = p {
                        if let syn::Expr::Lit(l) = &n.value {
                            if let syn::Lit::Str(l) = &l.lit {
                                let name = l.value();
                                let val: bool = m.check_field(&name, "true");
                                Some(val)
                            } else {
                                panic!("Expected a string literal");
                            }
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
                if let Some(a) = attr.path().get_ident() {
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

/// A macro that builds an iterator over all the variations of an enum
#[proc_macro_attribute]
pub fn vec_builder(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    assert!(attr.is_empty());
    let f = parse_macro_input!(item as syn::ItemEnum);
    let name = f.ident.clone();
    let fts = quote::ToTokens::into_token_stream(f.clone());
    let calls = f.variants.iter().map(|i| {
        let j = i.clone();
        let k = j.fields.iter().next().unwrap();
        quote! {
            #k::new().into()
        }
    });

    quote! {
        impl #name {
            /// Pushes one of every variant onto the given vector using the new function
            pub fn build_vec(b: &mut alloc::vec::Vec<Self>) {
                for d in [#(#calls),*] {
                    b.push(d);
                }
            }
        }
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
                crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!("Running test #{}: {}... ", #index, #fcall2));
                let r = #fcall();
                if r.is_err() {
                    crate::VGA.print_str("failed\r\n");
                }
                else {
                    crate::VGA.print_str("passed\r\n");
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
    let check = {
        let mut testa = TEST_CALL_QUANTITY.lock().unwrap();
        testa.take()
    };
    if let Some(testa) = check {
        let i = 0..testa;
        let calls = i.into_iter().map(|i| {
            let ident = quote::format_ident!("test_{}", i);
            quote!(Self::#ident)
        });

        quote! {
            impl DoorsTester {
                fn doors_test_main() -> Result<(),()> {
                    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!("Running all {} Doors tests\r\n", #testa));
                    #(#calls()?;)*
                    crate::VGA.print_fixed_str(doors_macros2::fixed_string_format!("All {} tests passed\r\n", #testa));
                    Ok(())
                }
            }
        }
        .into()
    } else {
        quote! {
            impl DoorsTester {
                fn doors_test_main() -> Result<(),()> {
                    crate::VGA.print_str("No Doors tests to run\r\n");
                    Ok(())
                }
            }
        }
        .into()
    }
}

#[cfg(feature = "backtrace")]
/// Used for instrumenting async functions for seeing backtraces
#[proc_macro_attribute]
pub fn framed(
    args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    assert!(args.is_empty());
    // Cloning a `TokenStream` is cheap since it's reference counted internally.
    backtrace::instrument_precise(item.clone())
        .unwrap_or_else(|_err| backtrace::instrument_speculative(item))
}

#[cfg(not(feature = "backtrace"))]
#[proc_macro_attribute]
/// Used for instrumenting async functions for seeing backtraces, this does nothing since backtraces are not enabled
pub fn framed(
    _args: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    item
}

//! Procedural macros for the aetos metrics library.
//!
//! This crate provides the `#[metrics]` attribute macro and `#[derive(Label)]` macro
//! for generating Prometheus metrics rendering code.
//!
//! ## Label Validation
//!
//! The `#[metrics]` macro validates that the `label` attribute is not used on common
//! scalar primitive types (u64, f64, i32, etc.). Using `label` on these types will
//! produce a compile error:
//!
//! ```text
//! error: the 'label' attribute is not supported on scalar types like u64, f64, etc.
//!        Labels are only supported on collection types that implement IntoIterator.
//! ```
//!
//! **Note:** This validation only catches common primitive types.
//! Type aliases and custom wrapper types are not detected and will silently ignore labels.
//!
//! See the main `aetos` crate documentation for usage examples.

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod label_derive;
mod metrics_macro;

#[proc_macro_derive(Label)]
pub fn derive_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    label_derive::expand_label_derive(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn metrics(args: TokenStream, input: TokenStream) -> TokenStream {
    metrics_macro::expand_metrics_macro(args.into(), input.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

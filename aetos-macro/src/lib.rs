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

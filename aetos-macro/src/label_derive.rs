use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Result};

pub fn expand_label_derive(input: DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(Error::new_spanned(
                    input,
                    "Label can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(Error::new_spanned(
                input,
                "Label can only be derived for structs",
            ));
        }
    };

    let field_idents: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let field_types: Vec<_> = fields.iter().map(|f| &f.ty).collect();

    let format_impl = if field_idents.is_empty() {
        quote! {
            Ok(())
        }
    } else {
        let first_field = &field_idents[0];
        let rest_fields = &field_idents[1..];

        quote! {
            write!(
                f,
                "{}=\"{}\"",
                stringify!(#first_field),
                ::aetos::core::escape_label_value(&self.#first_field.to_string())
            )?;
            #(
                write!(
                    f,
                    ",{}=\"{}\"",
                    stringify!(#rest_fields),
                    ::aetos::core::escape_label_value(&self.#rest_fields.to_string())
                )?;
            )*
            Ok(())
        }
    };

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let where_clause = if field_types.is_empty() {
        where_clause.cloned()
    } else {
        let mut where_clause = where_clause.cloned().unwrap_or_else(|| syn::WhereClause {
            where_token: Default::default(),
            predicates: Default::default(),
        });

        for ty in &field_types {
            where_clause
                .predicates
                .push(syn::parse_quote!(#ty: std::fmt::Display));
        }

        Some(where_clause)
    };

    Ok(quote! {
        impl #impl_generics ::aetos::core::Label for #name #ty_generics #where_clause {
            fn fmt_labels(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                #format_impl
            }
        }
    })
}

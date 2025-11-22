use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Expr, ExprLit, Fields, Lit, Meta, Result, parse2};

#[derive(Debug)]
enum MetricType {
    Counter,
    Gauge,
}

#[derive(Debug)]
enum FieldType {
    SingleLabel { label_name: Option<String> },
    Unspecified,
}

#[derive(Debug)]
struct MetricField {
    ident: syn::Ident,
    field_type: FieldType,
    metric_type: MetricType,
    help: String,
    name_override: Option<String>,
}

pub fn expand_metrics_macro(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let mut input: DeriveInput = parse2(input)?;
    let prefix = parse_struct_attrs(args)?;

    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(Error::new_spanned(
                    input,
                    "metrics can only be applied to structs with named fields",
                ));
            }
        },
        _ => {
            return Err(Error::new_spanned(
                input,
                "metrics can only be applied to structs",
            ));
        }
    };

    let mut metric_fields = Vec::new();

    for field in fields {
        if let Some(metric_field) = parse_field(field)? {
            metric_fields.push(metric_field);
        }
    }

    if let Data::Struct(ref mut data) = input.data
        && let Fields::Named(ref mut fields) = data.fields
    {
        for field in &mut fields.named {
            field
                .attrs
                .retain(|attr| !attr.path().is_ident("counter") && !attr.path().is_ident("gauge"));
        }
    }

    let original_struct = quote! {
        #input
    };

    let display_impl =
        generate_display_impl(name, &input.generics, &metric_fields, prefix.as_deref())?;

    let output = quote! {
        #original_struct
        #display_impl
    };
    Ok(output)
}

fn parse_struct_attrs(args: TokenStream) -> Result<Option<String>> {
    if args.is_empty() {
        return Ok(None);
    }

    let meta: Meta = parse2(args)?;

    match meta {
        Meta::List(list) => {
            for nested in list.parse_args_with(
                syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
            )? {
                if let Meta::NameValue(nv) = nested
                    && nv.path.is_ident("prefix")
                    && let Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }) = &nv.value
                {
                    return Ok(Some(s.value()));
                }
            }
        }
        Meta::NameValue(nv) => {
            if nv.path.is_ident("prefix")
                && let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
            {
                return Ok(Some(s.value()));
            }
        }
        _ => {}
    }

    Ok(None)
}

fn parse_field(field: &syn::Field) -> Result<Option<MetricField>> {
    let mut metric_type = None;
    let mut help = None;
    let mut name_override = None;
    let mut label_override = None;

    for attr in &field.attrs {
        if attr.path().is_ident("counter") {
            metric_type = Some(MetricType::Counter);
            parse_metric_attrs(attr, &mut help, &mut name_override, &mut label_override)?;
        } else if attr.path().is_ident("gauge") {
            metric_type = Some(MetricType::Gauge);
            parse_metric_attrs(attr, &mut help, &mut name_override, &mut label_override)?;
        }
    }

    let metric_type = match metric_type {
        Some(mt) => mt,
        None => return Ok(None),
    };

    let help = help.ok_or_else(|| {
        Error::new_spanned(field, "counter/gauge attribute requires 'help' parameter")
    })?;

    let ident = field.ident.as_ref().unwrap().clone();

    let field_type = match label_override {
        Some(label_name) => FieldType::SingleLabel {
            label_name: Some(label_name),
        },
        None => FieldType::Unspecified,
    };

    Ok(Some(MetricField {
        ident,
        field_type,
        metric_type,
        help,
        name_override,
    }))
}

fn parse_metric_attrs(
    attr: &syn::Attribute,
    help: &mut Option<String>,
    name_override: &mut Option<String>,
    label_override: &mut Option<String>,
) -> Result<()> {
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("help") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            *help = Some(s.value());
            Ok(())
        } else if meta.path.is_ident("name") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            *name_override = Some(s.value());
            Ok(())
        } else if meta.path.is_ident("label") {
            let value = meta.value()?;
            let s: syn::LitStr = value.parse()?;
            *label_override = Some(s.value());
            Ok(())
        } else {
            Err(meta.error("unknown attribute"))
        }
    })
}

fn generate_display_impl(
    name: &syn::Ident,
    generics: &syn::Generics,
    fields: &[MetricField],
    prefix: Option<&str>,
) -> Result<TokenStream> {
    let mut fmt_methods = Vec::new();
    let mut fmt_calls = Vec::new();

    for field in fields {
        let method_name = syn::Ident::new(&format!("fmt_{}", field.ident), field.ident.span());
        let field_ident = &field.ident;

        let metric_name = build_metric_name(&field.ident, field.name_override.as_deref(), prefix);
        let help = &field.help;
        let metric_type_str = match field.metric_type {
            MetricType::Counter => "counter",
            MetricType::Gauge => "gauge",
        };

        let method_impl = match &field.field_type {
            FieldType::SingleLabel { label_name } => {
                let label_name = label_name
                    .clone()
                    .unwrap_or_else(|| field.ident.to_string());

                quote! {
                    fn #method_name(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        use aetos::core::{MetricWrapper, MetricMetadata, RenderScalarFallback};

                        let meta = MetricMetadata {
                            name: #metric_name,
                            help: #help,
                            kind: #metric_type_str,
                        };

                        let wrapper = MetricWrapper(&self.#field_ident);
                        wrapper.render_with_label_attr(f, &meta, #label_name)
                    }
                }
            }
            FieldType::Unspecified => {
                quote! {
                    fn #method_name(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        use aetos::core::{MetricWrapper, MetricMetadata, RenderScalarFallback};

                        let meta = MetricMetadata {
                            name: #metric_name,
                            help: #help,
                            kind: #metric_type_str,
                        };

                        let wrapper = MetricWrapper(&self.#field_ident);
                        wrapper.render_with_struct_key(f, &meta)
                    }
                }
            }
        };

        fmt_methods.push(method_impl);
        fmt_calls.push(quote! { self.#method_name(f)?; });
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            #(#fmt_methods)*
        }

        impl #impl_generics std::fmt::Display for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                #(#fmt_calls)*
                Ok(())
            }
        }

        impl #impl_generics aetos::core::PrometheusMetric for #name #ty_generics #where_clause {}
    })
}

fn build_metric_name(
    ident: &syn::Ident,
    name_override: Option<&str>,
    prefix: Option<&str>,
) -> String {
    let ident_string = ident.to_string();
    let base_name = name_override.unwrap_or(&ident_string);

    match prefix {
        Some(p) => format!("{}_{}", p, base_name),
        None => base_name.to_string(),
    }
}

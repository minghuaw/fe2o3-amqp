use darling::FromDeriveInput;
use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, Lit, Meta};

use crate::{DescribedStructAttr, DescribedAttr, EncodingType};

pub(crate) fn parse_described_struct_attr(input: &syn::DeriveInput) -> DescribedStructAttr {
    let attr = DescribedAttr::from_derive_input(&input).unwrap();

    let name = attr.name.unwrap_or_else(|| input.ident.to_string());
    let code = attr.code;
    let encoding = attr.encoding.unwrap_or(EncodingType::List);
    let rename_field = attr.rename_all;
    DescribedStructAttr {
        name,
        code,
        encoding,
        rename_field,
    }
}



pub(crate) fn convert_to_case(
    case: &str,
    source: String,
    ctx: &DeriveInput,
) -> Result<String, syn::Error> {
    use convert_case::{Case, Casing};
    let s = match case {
        "" => source,
        "lowercase" => source.to_lowercase(),
        "UPPERCASE" => source.to_uppercase(),
        "PascalCase" => source.to_case(Case::Pascal),
        "camelCase" => source.to_case(Case::Camel),
        "snake_case" => source.to_case(Case::Snake),
        "SCREAMING_SNAKE_CASE" => source.to_case(Case::ScreamingSnake),
        "kebab-case" => source.to_case(Case::Kebab),
        e @ _ => {
            let span = get_span_of("rename_all", ctx);
            match span {
                Some(span) => {
                    return Err(syn::Error::new(
                        span,
                        format!("{} case is not implemented", e),
                    ))
                }
                None => {
                    return Err(syn::Error::new(
                        ctx.ident.span(),
                        format!("{} case is not implemented", e),
                    ))
                }
            }
        }
    };

    Ok(s)
}

pub(crate) fn get_span_of(ident_str: &str, ctx: &DeriveInput) -> Option<Span> {
    ctx.attrs
        .iter()
        .find_map(|attr| match attr.path.get_ident() {
            Some(i) => {
                if i.to_string() == ident_str {
                    Some(i.span())
                } else {
                    None
                }
            }
            None => None,
        })
}

/// Buffer the Null (for None value)
pub(crate) fn macro_rules_buffer_if_none() -> proc_macro2::TokenStream {
    quote! {
        macro_rules! buffer_if_none {
            // for tuple struct
            ($state: ident, $nulls: ident, $fident: expr, Option<$ftype: ty>) => {
                if $fident.is_some() {
                    for _ in 0..$nulls {
                        $state.serialize_field(&())?; // `None` and `()` share the same encoding
                    }
                    $nulls = 0;
                    $state.serialize_field($fident)?;
                } else {
                    $nulls += 1;
                }
            };
            ($state: ident, $nulls: ident, $fident: expr, $ftype: ty) => {
                for _ in 0..$nulls {
                    $state.serialize_field(&())?; // `None` and `()` share the same encoding
                }
                $nulls = 0;
                $state.serialize_field($fident)?;
            };

            // for struct
            ($state: ident, $nulls: ident, $fident: expr, $fname: expr, Option<$ftype: ty>) => {
                if $fident.is_some() {
                    for _ in 0..$nulls {
                        // name is not used in list encoding
                        $state.serialize_field("", &())?; // `None` and `()` share the same encoding
                    }
                    $nulls = 0;
                    $state.serialize_field($fname, $fident)?;
                } else {
                    $nulls += 1;
                }
            };
            ($state: ident, $nulls: ident, $fident: expr, $fname: expr, $ftype: ty) => {
                for _ in 0..$nulls {
                    // name is not used in list encoding
                    $state.serialize_field("", &())?; // `None` and `()` share the same encoding
                }
                $nulls = 0;
                $state.serialize_field($fname, $fident)?;
            };
        }
    }
}

pub(crate) fn macro_rules_unwrap_or_none() -> proc_macro2::TokenStream {
    quote! {
        macro_rules! unwrap_or_none {
            ($fident: ident, $seq: expr, Option<$ftype: ty>) => {
                let $fident: Option<$ftype> = match $seq {
                    Some(val) => val,
                    None => None,
                };
            };
            ($fident: ident, $seq: ident, $ftype: ty) => {
                let $fident: $ftype = match $seq {
                    Some(val) => val,
                    None => return Err(fe2o3_amqp::serde::de::Error::custom("Insufficient number of items")),
                };
            };
        }
    }
}

pub(crate) fn macro_rules_unwrap_or_default() -> proc_macro2::TokenStream {
    quote! {
        macro_rules! unwrap_or_default {
            ($fident: ident, $seq: expr, $ftype: ty, $default: expr) => {
                let $fident: $ftype = match $seq {
                    Some(val) => val,
                    None => $default,
                };
            };
        }
    }
}
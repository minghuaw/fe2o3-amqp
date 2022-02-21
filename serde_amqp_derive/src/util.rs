use darling::{FromDeriveInput, FromMeta};
use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, Field, parse::Parser};

use crate::{DescribedAttr, DescribedStructAttr, EncodingType, FieldAttr};

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

pub(crate) fn parse_named_field_attrs<'a>(
    fields: impl Iterator<Item = &'a Field>,
) -> Vec<FieldAttr> {
    fields
        .map(|f| {
            f.attrs.iter().find_map(|a| {
                let item = a.parse_meta().unwrap();
                FieldAttr::from_meta(&item).ok()
            })
        })
        .map(|o| o.unwrap_or_else(|| FieldAttr { default: false }))
        .collect()
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

pub(crate) fn where_serialize(generics: &syn::Generics) -> proc_macro2::TokenStream {
    let mut wheres = Vec::new();
    generics.params.iter()
        .filter_map(|param| {
            if let syn::GenericParam::Type(tparam) = param {
                Some(&tparam.ident)
            } else {
                None
            }
        })
        .for_each(|id| {
            wheres.push(quote!{
                #id: serde::ser::Serialize
            })
        });
    quote! {
        where
            #(#wheres),*
    }
}

pub(crate) fn where_deserialize(generics: &syn::Generics) -> proc_macro2::TokenStream {
    let mut wheres = Vec::new();
    generics.params.iter()
        .filter_map(|param| {
            if let syn::GenericParam::Type(tparam) = param {
                Some(&tparam.ident)
            } else {
                None
            }
        })
        .for_each(|id| {
            wheres.push(quote!{
                #id: serde::de::Deserialize<'de>
            })
        });
    quote! {
        where
            #(#wheres),*
    }
}

pub(crate) fn generic_visitor(generics: &syn::Generics) -> proc_macro2::TokenStream {
    let (generic_types, fields) = generic_visitor_fields(generics);
    let field_ids: Vec<syn::Ident> = fields.iter()
        .map(|f| {
            f.ident.clone().unwrap()
        })
        .collect();

    quote! {
        struct Visitor<#(#generic_types),*> {
            #(#fields),*
        }

        impl<#(#generic_types),*> Visitor<#(#generic_types),*> {
            fn new() -> Self {
                Self {
                    #(#field_ids: std::marker::PhantomData),*
                }
            }
        }
    }
}

pub(crate) fn generic_visitor_fields(generics: &syn::Generics) -> (Vec<&syn::Ident>, Vec<syn::Field>) {
    let mut types: Vec<&syn::Ident> = Vec::new();
    let mut fields: Vec<syn::Field> = Vec::new();
    generics.params.iter()
        .filter_map(|param| {
            if let syn::GenericParam::Type(tparam) = param {
                Some(&tparam.ident)
            } else {
                None
            }
        })
        .enumerate()
        .for_each(|(i, ty)| {
            types.push(ty);
            let field_id = syn::Ident::new(&format!("_field{}", i), ty.span());
            let token = quote!(#field_id: std::marker::PhantomData<#ty>);
            let field = syn::Field::parse_named.parse2(token);
            fields.push(field.unwrap());
        });
    (types, fields)
}

pub(crate) fn macro_rules_buffer_if_none_for_tuple_struct() -> proc_macro2::TokenStream {
    quote! {
        macro_rules! buffer_if_none_for_tuple {
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
        }
    }
}

/// Buffer the Null (for None value)
pub(crate) fn macro_rules_buffer_if_none() -> proc_macro2::TokenStream {
    quote! {
        macro_rules! buffer_if_none {
            // for struct
            ($state: ident, $nulls: ident, $fident: expr, $fname: expr, Option<$ftype: ty>) => {
                if $fident.is_some() {
                    for field_name in $nulls.drain(..) {
                        // name is not used in list encoding
                        $state.serialize_field(field_name, &())?; // `None` and `()` share the same encoding
                    }
                    $state.serialize_field($fname, $fident)?;
                } else {
                    $nulls.push($fname);
                }
            };
            ($state: ident, $nulls: ident, $fident: expr, $fname: expr, $ftype: ty) => {
                for field_name in $nulls.drain(..) {
                    // name is not used in list encoding
                    $state.serialize_field(field_name, &())?; // `None` and `()` share the same encoding
                }
                $state.serialize_field($fname, $fident)?;
            };
        }
    }
}

/// Buffer the Null (for None value)
pub(crate) fn macro_rules_buffer_if_eq_default() -> proc_macro2::TokenStream {
    quote! {
        macro_rules! buffer_if_eq_default {
            // for struct
            ($state: ident, $nulls: ident, $fident: expr, $fname: expr, $ftype: ty) => {
                if *$fident != <$ftype as Default>::default() {
                    // Only serialize if value is not equal to default
                    for field_name in $nulls.drain(..) {
                        // name is not used in list encoding
                        $state.serialize_field(field_name, &())?; // `None` and `()` share the same encoding
                    }
                    $state.serialize_field($fname, $fident)?;
                } else {
                    $nulls.push($fname);
                }
            };
        }
    }
}

pub(crate) fn macro_rules_serialize_if_some() -> proc_macro2::TokenStream {
    quote! {
        macro_rules! serialize_if_some {
            // for struct
            ($state: ident, $fident: expr, $fname: expr, Option<$ftype: ty>) => {
                if $fident.is_some() {
                    $state.serialize_field($fname, $fident)?;
                }
            };
            ($state: ident, $fident: expr, $fname: expr, $ftype: ty) => {
                $state.serialize_field($fname, $fident)?;
            };
        }
    }
}

pub(crate) fn macro_rules_serialize_if_neq_default() -> proc_macro2::TokenStream {
    quote! {
        macro_rules! serialize_if_neq_default {
            // for struct
            ($state: ident, $fident: expr, $fname: expr, $ftype: ty) => {
                if *$fident != <$ftype as Default>::default() {
                    $state.serialize_field($fname, $fident)?;
                }
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
            ($fident: ident, $seq: expr, $ftype: ty) => {
                let $fident: $ftype = match $seq {
                    Some(val) => val,
                    None => return Err(serde_amqp::serde::de::Error::custom("Insufficient number of items")),
                };
            };
        }
    }
}

pub(crate) fn macro_rules_unwrap_or_default() -> proc_macro2::TokenStream {
    quote! {
        macro_rules! unwrap_or_default {
            ($fident: ident, $seq: expr, $ftype: ty) => {
                let $fident: $ftype = match $seq {
                    Some(val) => val,
                    None => Default::default(),
                };
            };
        }
    }
}
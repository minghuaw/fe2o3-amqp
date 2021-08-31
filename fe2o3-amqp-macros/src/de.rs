use quote::quote;
use syn::{DeriveInput, Fields, spanned::Spanned};

use crate::{AmqpContractAttr, EncodingType, util::{convert_to_case, get_span_of, macro_rules_unwrap_or_none, parse_described_attr}};

pub(crate) fn expand_deserialize(
    input: &syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let attr = parse_described_attr(input);
    let ident = &input.ident;
    match &input.data {
        syn::Data::Struct(data) => expand_deserialize_on_datastruct(&attr, ident, data, input),
        _ => unimplemented!(),
    }
}

fn expand_deserialize_on_datastruct(
    attr: &AmqpContractAttr,
    ident: &syn::Ident,
    data: &syn::DataStruct,
    ctx: &DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let name = &attr.name[..]; // descriptor name
    let expecting = format!("struct {}", name);

    let evaluate_code = match attr.code {
        Some(code) => quote! {
            fe2o3_amqp::types::Descriptor::Code(c) => {
                if c != #code {
                    return Err(fe2o3_amqp::serde::de::Error::custom("Descriptor mismatch"))
                }
            }
        },
        None => quote! {
            fe2o3_amqp::types::Descriptor::Code(_) => return Err(fe2o3_amqp::serde::de::Error::custom("Descriptor mismatch"))
        },
    };

    let evaluate_descriptor = quote! {
        match descriptor {
            fe2o3_amqp::types::Descriptor::Name(symbol) => {
                if symbol.into_inner() != #name {
                    return Err(fe2o3_amqp::serde::de::Error::custom("Descriptor mismatch"))
                }
            },
            #evaluate_code
        }
    };

    match &data.fields {
        Fields::Named(fields) => {
            Ok(expand_deserialize_struct(ident, &expecting, &evaluate_descriptor, &attr.encoding, &attr.rename_field, fields, ctx)?)
        },
        Fields::Unnamed(fields) => {
            Ok(expand_deserialize_tuple_struct(ident, name, &evaluate_descriptor, &attr.encoding, fields, ctx)?)
        },
        Fields::Unit => {
            Ok(expand_deserialize_unit_struct(ident, &expecting, &evaluate_descriptor, &attr.encoding, ctx)?)
        }
    }
}


fn impl_visit_seq_for_unit_struct(
    ident: &syn::Ident,
    evaluate_descriptor: &proc_macro2::TokenStream, 
) -> proc_macro2::TokenStream {
    quote! {
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: fe2o3_amqp::serde::de::SeqAccess<'de>,
        {
            let descriptor: fe2o3_amqp::types::Descriptor = match seq.next_element()? {
                Some(val) => val,
                None => return Err(fe2o3_amqp::serde::de::Error::custom("Expecting descriptor"))
            };

            #evaluate_descriptor

            Ok( #ident )
        }
    }
}

fn expand_deserialize_unit_struct(
    ident: &syn::Ident, 
    expecting: &str, 
    evaluate_descriptor: &proc_macro2::TokenStream, 
    encoding: &EncodingType,
    ctx: &DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let struct_name = match encoding {
        EncodingType::List => quote!(fe2o3_amqp::constants::DESCRIBED_LIST),
        EncodingType::Basic => {
            let span = get_span_of("encoding", ctx).unwrap_or(ident.span());
            return Err(syn::Error::new(span, "Basic encoding is not supported for unit struct"))
        },
        EncodingType::Map => {
            let span = get_span_of("encoding", ctx).unwrap_or(ident.span());
            return Err(syn::Error::new(span, "Map encoding is not supported for unit struct"))
        },
    };
    let visit_seq = impl_visit_seq_for_unit_struct(ident, evaluate_descriptor);
    let len = 0usize;

    let token = quote! {
        #[automatically_derived]
        impl<'de> fe2o3_amqp::serde::de::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: fe2o3_amqp::serde::de::Deserializer<'de>,
            {
                struct Visitor {}
                impl<'de> fe2o3_amqp::serde::de::Visitor<'de> for Visitor {
                    type Value = #ident;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str(#expecting)
                    }

                    #visit_seq            
                }

                // DESCRIPTOR is included here for compatibility with other deserializer
                deserializer.deserialize_tuple_struct(
                    #struct_name,
                    #len,
                    Visitor{}
                )
            }
        }
    };
    Ok(token)
}

fn impl_visit_seq_for_tuple_struct(
    ident: &syn::Ident,
    field_idents: &Vec<syn::Ident>,
    field_types: &Vec<&syn::Type>,
    evaluate_descriptor: &proc_macro2::TokenStream, 
) -> proc_macro2::TokenStream {
    quote! {
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: fe2o3_amqp::serde::de::SeqAccess<'de>,
        {
            let descriptor: fe2o3_amqp::types::Descriptor = match seq.next_element()? {
                Some(val) => val,
                None => return Err(fe2o3_amqp::serde::de::Error::custom("Expecting descriptor"))
            };

            #evaluate_descriptor

            #(let #field_idents: #field_types = match seq.next_element()? {
                Some(val) => val,
                None => return Err(fe2o3_amqp::serde::de::Error::custom("Invalid length"))
            };)*

            Ok( #ident( #(#field_idents, )* ) )
        }
    }
}

fn expand_deserialize_tuple_struct(
    ident: &syn::Ident, 
    expecting: &str,
    evaluate_descriptor: &proc_macro2::TokenStream, 
    encoding: &EncodingType,
    fields: &syn::FieldsUnnamed,
    ctx: &DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let struct_name = match encoding {
        EncodingType::List => quote!(fe2o3_amqp::constants::DESCRIBED_LIST),
        EncodingType::Basic => {
            let span = get_span_of("encoding", ctx).unwrap_or(ident.span());
            return Err(syn::Error::new(span, "Basic encoding is not supported for tuple struct"))
        },
        EncodingType::Map => {
            let span = get_span_of("encoding", ctx).unwrap_or(ident.span());
            return Err(syn::Error::new(span, "Map encoding is not supported for tuple struct"))
        },
    };
    let field_idents: Vec<syn::Ident> = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, f)| (format!("field{}", i), f.span()))
        .map(|(id, span)| syn::Ident::new(&id, span))
        .collect();
    
    let field_types: Vec<&syn::Type> = fields.unnamed.iter().map(|f| &f.ty).collect();
    let visit_seq = impl_visit_seq_for_tuple_struct(ident, &field_idents, &field_types, evaluate_descriptor);
    let len = field_idents.len();

    let token = quote! {
        #[automatically_derived]
        impl<'de> fe2o3_amqp::serde::de::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: fe2o3_amqp::serde::de::Deserializer<'de>,
            {
                struct Visitor {}
                impl<'de> fe2o3_amqp::serde::de::Visitor<'de> for Visitor {
                    type Value = #ident;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str(#expecting)
                    }

                    #visit_seq            
                }

                // DESCRIPTOR is included here for compatibility with other deserializer
                deserializer.deserialize_tuple_struct(
                    #struct_name,
                    #len,
                    Visitor{}
                )
            }
        }
    };
    Ok(token)
}

fn expand_deserialize_struct(
    ident: &syn::Ident, 
    expecting: &str,
    evaluate_descriptor: &proc_macro2::TokenStream, 
    encoding: &EncodingType,
    rename_field: &str,
    fields: &syn::FieldsNamed,
    ctx: &DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let len = fields.named.len();
    let struct_name = match encoding {
        EncodingType::List => quote!(fe2o3_amqp::constants::DESCRIBED_LIST),
        EncodingType::Basic => {
            match len {
                0 => {
                    let span = get_span_of("encoding", ctx).unwrap_or(ident.span());
                    return Err(syn::Error::new(span, "Basic encoding is not supported on unit struct"))
                },
                _ => quote!(fe2o3_amqp::constants::DESCRIBED_BASIC)
            }
        },
        EncodingType::Map => {
            match len {
                0 => {
                    let span = get_span_of("encoding", ctx).unwrap_or(ident.span());
                    return Err(syn::Error::new(span, "Map encoding on unit struct is not implemented"))
                }
                _ => quote!(fe2o3_amqp::constants::DESCRIBED_MAP)
            }
        },
    };
    let field_idents: Vec<syn::Ident> = fields.named
        .iter()
        .map(|f| f.ident.clone().unwrap())
        .collect();
    let field_names: Vec<String> = field_idents
        .iter()
        .map(|i| convert_to_case(rename_field, i.to_string(), ctx).unwrap())
        .collect();
    let field_types: Vec<&syn::Type> = fields.named.iter().map(|f| &f.ty).collect();

    let deserialize_field = impl_deserialize_for_field(&field_idents, &field_names);

    let unwrap_or_none = macro_rules_unwrap_or_none();
    let visit_seq = impl_visit_seq_for_struct(ident, &field_idents, &field_types, evaluate_descriptor);
    let visit_map = match len {
        0 => quote!{},
        _ => impl_visit_map(ident, &field_idents, &field_names, &field_types, evaluate_descriptor)
    };

    let token = quote! {
        #unwrap_or_none

        #[automatically_derived]
        impl<'de> fe2o3_amqp::serde::de::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: fe2o3_amqp::serde::de::Deserializer<'de>,
            {

                #deserialize_field

                struct Visitor {}
                impl<'de> fe2o3_amqp::serde::de::Visitor<'de> for Visitor {
                    type Value = #ident;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str(#expecting)
                    }

                    #visit_seq            

                    #visit_map
                }

                // DESCRIPTOR is included here for compatibility with other deserializer
                const FIELDS: &'static [&'static str] = &[fe2o3_amqp::constants::DESCRIPTOR, #(#field_names,)*];
                deserializer.deserialize_struct(
                    #struct_name,
                    FIELDS,
                    Visitor{}
                )
            }
        }
    };
    Ok(token)
}

fn impl_deserialize_for_field(
    field_idents: &Vec<syn::Ident>,
    field_names: &Vec<String>,
) -> proc_macro2::TokenStream {
    quote! {
        #[allow(non_camel_case_types)]
        enum Field {
            #(#field_idents, )*
            // TODO: considering add ignored
        }
        struct FieldVisitor {}
        impl<'de> fe2o3_amqp::serde::de::Visitor<'de> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("field identifier")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: fe2o3_amqp::serde::de::Error,
            {
                match v {
                    // #name => Ok(Self::Value::descriptor),
                    #(#field_names => Ok(Self::Value::#field_idents),)*
                    _ => Err(fe2o3_amqp::serde::de::Error::custom("Unknown identifier"))
                }
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: fe2o3_amqp::serde::de::Error,
            {
                match v {
                    // b if b == #name.as_bytes() => Ok(Self::Value::descriptor),
                    #(b if b == #field_names.as_bytes() => Ok(Self::Value::#field_idents),)*
                    _ => Err(fe2o3_amqp::serde::de::Error::custom("Unknown identifier"))
                }
            }

        }
        impl<'de> fe2o3_amqp::serde::de::Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: fe2o3_amqp::serde::de::Deserializer<'de>,
            {
                deserializer.deserialize_identifier(FieldVisitor{})
            }
        }
    }
}

fn impl_visit_seq_for_struct(
    ident: &syn::Ident,
    field_idents: &Vec<syn::Ident>,
    field_types: &Vec<&syn::Type>,
    evaluate_descriptor: &proc_macro2::TokenStream, 
) -> proc_macro2::TokenStream {
    quote! {
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: fe2o3_amqp::serde::de::SeqAccess<'de>,
        {
            let descriptor: fe2o3_amqp::types::Descriptor = match seq.next_element()? {
                Some(val) => val,
                None => return Err(fe2o3_amqp::serde::de::Error::custom("Expecting descriptor"))
            };

            #evaluate_descriptor

            // #(let #field_idents: #field_types = match seq.next_element()? {
            //     Some(val) => val,
            //     None => return Err(fe2o3_amqp::serde::de::Error::custom("Invalid length"))
            // };)*
            #( unwrap_or_none!(#field_idents, seq, #field_types); )*

            Ok( #ident{ #(#field_idents, )* } )
        }
    }
}

fn impl_visit_map(
    ident: &syn::Ident,
    field_idents: &Vec<syn::Ident>,
    field_names: &Vec<String>,
    field_types: &Vec<&syn::Type>,
    evaluate_descriptor: &proc_macro2::TokenStream, 
) -> proc_macro2::TokenStream {
    quote! {
        fn visit_map<A>(self, mut map: A)-> Result<Self::Value, A::Error>
        where A: fe2o3_amqp::serde::de::MapAccess<'de>
        {
            #(let mut #field_idents: Option<#field_types> = None;)*

            // The first should always be the descriptor
            let descriptor: fe2o3_amqp::types::Descriptor = match map.next_key()? {
                Some(val) => val,
                None => return Err(fe2o3_amqp::serde::de::Error::custom("Expecting descriptor"))
            };

            #evaluate_descriptor

            while let Some(key) = map.next_key::<Field>()? {
                match key {
                    #(
                        Field::#field_idents => {
                            if #field_idents.is_some() {
                                return Err(fe2o3_amqp::serde::de::Error::duplicate_field(#field_names))
                            }
                            #field_idents = Some(map.next_value()?);
                        },
                    )*
                }
            }

            #(
                let #field_idents = match #field_idents {
                    Some(val) => val,
                    None => return Err(fe2o3_amqp::serde::de::Error::missing_field(#field_names))
                };
            )*
            Ok( #ident{ #(#field_idents, )* } )
        }
    }
}
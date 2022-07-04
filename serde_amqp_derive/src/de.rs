use quote::quote;
use syn::{spanned::Spanned, DeriveInput, Fields};

use crate::{
    util::{
        convert_to_case, generic_visitor, get_span_of, macro_rules_unwrap_or_default,
        macro_rules_unwrap_or_none, parse_described_struct_attr, parse_named_field_attrs,
        where_deserialize,
    },
    DescribedStructAttr, EncodingType, FieldAttr,
};

pub(crate) fn expand_deserialize(
    input: &syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let attr = parse_described_struct_attr(input);
    let ident = &input.ident;
    let generics = &input.generics;
    match &input.data {
        syn::Data::Struct(data) => {
            expand_deserialize_on_datastruct(&attr, ident, generics, data, input)
        }
        _ => unimplemented!(),
    }
}

fn expand_deserialize_on_datastruct(
    attr: &DescribedStructAttr,
    ident: &syn::Ident,
    generics: &syn::Generics,
    data: &syn::DataStruct,
    ctx: &DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let name = &attr.name[..]; // descriptor name
    let expecting = format!("struct {}", name);

    let evaluate_code = match attr.code {
        Some(code) => quote! {
            serde_amqp::descriptor::Descriptor::Code(__c) => {
                if __c != #code {
                    return Err(serde_amqp::serde::de::Error::custom("Descriptor mismatch"))
                }
            }
        },
        None => quote! {
            serde_amqp::descriptor::Descriptor::Code(_) => return Err(serde_amqp::serde::de::Error::custom("Descriptor mismatch"))
        },
    };

    let evaluate_descriptor = quote! {
        match __descriptor {
            serde_amqp::descriptor::Descriptor::Name(__symbol) => {
                if __symbol.into_inner() != #name {
                    return Err(serde_amqp::serde::de::Error::custom("Descriptor mismatch"))
                }
            },
            #evaluate_code
        }
    };

    match &data.fields {
        Fields::Named(fields) => Ok(expand_deserialize_struct(
            ident,
            generics,
            &expecting,
            &evaluate_descriptor,
            &attr.encoding,
            &attr.rename_field,
            fields,
            ctx,
        )?),
        Fields::Unnamed(fields) => Ok(expand_deserialize_tuple_struct(
            ident,
            generics,
            name,
            &evaluate_descriptor,
            &attr.encoding,
            fields,
            ctx,
        )?),
        Fields::Unit => Ok(expand_deserialize_unit_struct(
            ident,
            &expecting,
            &evaluate_descriptor,
            &attr.encoding,
            ctx,
        )?),
    }
}

fn impl_visit_seq_for_unit_struct(
    ident: &syn::Ident,
    evaluate_descriptor: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        fn visit_seq<A>(self, mut __seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde_amqp::serde::de::SeqAccess<'de>,
        {
            let __descriptor: serde_amqp::descriptor::Descriptor = match __seq.next_element()? {
                Some(__val) => __val,
                None => return Err(serde_amqp::serde::de::Error::custom("Expecting descriptor"))
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
        EncodingType::List => quote!(serde_amqp::__constants::DESCRIBED_LIST),
        EncodingType::Basic => {
            let span = get_span_of("encoding", ctx).unwrap_or(ident.span());
            return Err(syn::Error::new(
                span,
                "Basic encoding is not supported for unit struct",
            ));
        }
        EncodingType::Map => {
            let span = get_span_of("encoding", ctx).unwrap_or(ident.span());
            return Err(syn::Error::new(
                span,
                "Map encoding is not supported for unit struct",
            ));
        }
    };
    let visit_seq = impl_visit_seq_for_unit_struct(ident, evaluate_descriptor);
    let len = 0usize;

    let token = quote! {
        #[automatically_derived]
        impl<'de> serde_amqp::serde::de::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde_amqp::serde::de::Deserializer<'de>,
            {
                struct Visitor {}
                impl<'de> serde_amqp::serde::de::Visitor<'de> for Visitor {
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
    let unwrap_or_none = match field_idents.len() {
        0 => quote! {},
        _ => macro_rules_unwrap_or_none(),
    };
    quote! {
        fn visit_seq<A>(self, mut __seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde_amqp::serde::de::SeqAccess<'de>,
        {
            let __descriptor: serde_amqp::descriptor::Descriptor = match __seq.next_element()? {
                Some(val) => val,
                None => return Err(serde_amqp::serde::de::Error::custom("Expecting descriptor"))
            };

            #evaluate_descriptor

            #unwrap_or_none

            #( unwrap_or_none!(#field_idents, __seq.next_element()?, #field_types); )*

            Ok( #ident( #(#field_idents, )* ) )
        }
    }
}

fn expand_deserialize_tuple_struct(
    ident: &syn::Ident,
    generics: &syn::Generics,
    expecting: &str,
    evaluate_descriptor: &proc_macro2::TokenStream,
    encoding: &EncodingType,
    fields: &syn::FieldsUnnamed,
    ctx: &DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let struct_name = match encoding {
        EncodingType::List => quote!(serde_amqp::__constants::DESCRIBED_LIST),
        EncodingType::Basic => {
            if fields.unnamed.len() == 1 {
                quote!(serde_amqp::__constants::DESCRIBED_BASIC)
            } else {
                let span = get_span_of("encoding", ctx).unwrap_or(ident.span());
                return Err(syn::Error::new(
                    span,
                    "Basic encoding is not supported for tuple struct",
                ));
            }
        }
        EncodingType::Map => {
            let span = get_span_of("encoding", ctx).unwrap_or(ident.span());
            return Err(syn::Error::new(
                span,
                "Map encoding is not supported for tuple struct",
            ));
        }
    };
    let field_idents: Vec<syn::Ident> = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, f)| (format!("field{}", i), f.span()))
        .map(|(id, span)| syn::Ident::new(&id, span))
        .collect();

    let field_types: Vec<&syn::Type> = fields.unnamed.iter().map(|f| &f.ty).collect();
    let visit_seq =
        impl_visit_seq_for_tuple_struct(ident, &field_idents, &field_types, evaluate_descriptor);
    let len = field_idents.len();

    let gen_params = &generics.params;
    let visitor = generic_visitor(generics);
    let where_clause = where_deserialize(generics);

    let token = quote! {
        #[automatically_derived]
        impl<'de, #gen_params> serde_amqp::serde::de::Deserialize<'de> for #ident<#gen_params> #where_clause {
            fn deserialize<_D>(deserializer: _D) -> Result<Self, _D::Error>
            where
                _D: serde_amqp::serde::de::Deserializer<'de>,
            {
                // struct Visitor {}
                #visitor
                impl<'de, #gen_params> serde_amqp::serde::de::Visitor<'de> for Visitor<#gen_params> #where_clause {
                    type Value = #ident<#gen_params>;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str(#expecting)
                    }

                    #visit_seq
                }

                // DESCRIPTOR is included here for compatibility with other deserializer
                deserializer.deserialize_tuple_struct(
                    #struct_name,
                    #len + 1, // descriptor also takes one
                    Visitor::new()
                )
            }
        }
    };
    Ok(token)
}

fn expand_deserialize_struct(
    ident: &syn::Ident,
    generics: &syn::Generics,
    expecting: &str,
    evaluate_descriptor: &proc_macro2::TokenStream,
    encoding: &EncodingType,
    rename_all: &str,
    fields: &syn::FieldsNamed,
    ctx: &DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let len = fields.named.len();
    let struct_name = match encoding {
        EncodingType::List => quote!(serde_amqp::__constants::DESCRIBED_LIST),
        EncodingType::Basic => match len {
            0 => {
                let span = get_span_of("encoding", ctx).unwrap_or(ident.span());
                return Err(syn::Error::new(
                    span,
                    "Basic encoding is not supported on unit struct",
                ));
            }
            _ => quote!(serde_amqp::__constants::DESCRIBED_BASIC),
        },
        EncodingType::Map => match len {
            0 => {
                let span = get_span_of("encoding", ctx).unwrap_or(ident.span());
                return Err(syn::Error::new(
                    span,
                    "Map encoding on unit struct is not implemented",
                ));
            }
            _ => quote!(serde_amqp::__constants::DESCRIBED_MAP),
        },
    };
    let field_idents: Vec<syn::Ident> = fields
        .named
        .iter()
        .map(|f| f.ident.clone().unwrap())
        .collect();
    let field_names: Vec<String> = field_idents
        .iter()
        .map(|i| convert_to_case(rename_all, i.to_string(), ctx).unwrap())
        .collect();
    let field_types: Vec<&syn::Type> = fields.named.iter().map(|f| &f.ty).collect();
    let field_attrs = parse_named_field_attrs(fields.named.iter());

    let deserialize_field = impl_deserialize_for_field(&field_idents, &field_names);

    let visit_seq = impl_visit_seq_for_struct(
        ident,
        &field_idents,
        &field_types,
        &field_attrs,
        evaluate_descriptor,
    );
    let visit_map = match len {
        0 => quote! {},
        _ => impl_visit_map(
            ident,
            &field_idents,
            &field_names,
            &field_types,
            &field_attrs,
            evaluate_descriptor,
        ),
    };

    let mut n_true: u32 = 0;
    let mut n_false: u32 = 0;
    field_attrs.iter().for_each(|a| {
        if a.default == true {
            n_true += 1;
        } else {
            n_false += 1;
        }
    });

    let unwrap_or_default = match n_true {
        0 => quote! {},
        _ => macro_rules_unwrap_or_default(),
    };
    let unwrap_or_none = match n_false {
        0 => quote! {},
        _ => macro_rules_unwrap_or_none(),
    };
    let gen_params = &generics.params;
    let visitor = generic_visitor(generics);
    let where_clause = where_deserialize(generics);

    let token = quote! {
        #unwrap_or_default
        #unwrap_or_none

        #[automatically_derived]
        impl<'de, #gen_params> serde_amqp::serde::de::Deserialize<'de> for #ident<#gen_params> #where_clause {
            fn deserialize<_D>(deserializer: _D) -> Result<Self, _D::Error>
            where
                _D: serde_amqp::serde::de::Deserializer<'de>,
            {

                #deserialize_field

                #visitor
                impl<'de, #gen_params> serde_amqp::serde::de::Visitor<'de> for Visitor<#gen_params> #where_clause {
                    type Value = #ident<#gen_params>;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str(#expecting)
                    }

                    #visit_seq

                    #visit_map
                }

                // DESCRIPTOR is included here for compatibility with other deserializer
                const FIELDS: &'static [&'static str] = &[serde_amqp::__constants::DESCRIPTOR, #(#field_names,)*];
                deserializer.deserialize_struct(
                    #struct_name,
                    FIELDS,
                    Visitor::new()
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
        impl<'de> serde_amqp::serde::de::Visitor<'de> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("field identifier")
            }

            fn visit_str<_E>(self, v: &str) -> Result<Self::Value, _E>
            where
                _E: serde_amqp::serde::de::Error,
            {
                match v {
                    // #name => Ok(Self::Value::descriptor),
                    #(#field_names => Ok(Self::Value::#field_idents),)*
                    _ => Err(serde_amqp::serde::de::Error::custom("Unknown identifier"))
                }
            }

            fn visit_bytes<_E>(self, v: &[u8]) -> Result<Self::Value, _E>
            where
                _E: serde_amqp::serde::de::Error,
            {
                match v {
                    // b if b == #name.as_bytes() => Ok(Self::Value::descriptor),
                    #(b if b == #field_names.as_bytes() => Ok(Self::Value::#field_idents),)*
                    _ => Err(serde_amqp::serde::de::Error::custom("Unknown identifier"))
                }
            }

        }
        impl<'de> serde_amqp::serde::de::Deserialize<'de> for Field {
            fn deserialize<_D>(deserializer: _D) -> Result<Self, _D::Error>
            where
                _D: serde_amqp::serde::de::Deserializer<'de>,
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
    field_attrs: &Vec<FieldAttr>,
    evaluate_descriptor: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let mut field_impls: Vec<proc_macro2::TokenStream> = vec![];
    for ((id, ty), attr) in field_idents.iter().zip(field_types.iter()).zip(field_attrs) {
        let token = match attr.default {
            true => {
                quote! { unwrap_or_default!(#id, __seq.next_element::<Option<#ty>>()?.unwrap_or_default(), #ty) }
            }
            false => {
                quote! { unwrap_or_none!(#id, __seq.next_element()?, #ty) }
            }
        };
        field_impls.push(token);
    }

    quote! {
        fn visit_seq<_A>(self, mut __seq: _A) -> Result<Self::Value, _A::Error>
        where
            _A: serde_amqp::serde::de::SeqAccess<'de>,
        {
            let __descriptor: serde_amqp::descriptor::Descriptor = match __seq.next_element()? {
                Some(val) => val,
                None => return Err(serde_amqp::serde::de::Error::custom("Expecting descriptor"))
            };

            #evaluate_descriptor

            // #( unwrap_or_none!(#field_idents, __seq, #field_types); )*
            #( #field_impls; )*

            Ok( #ident{ #(#field_idents, )* } )
        }
    }
}

fn impl_visit_map(
    ident: &syn::Ident,
    field_idents: &Vec<syn::Ident>,
    field_names: &Vec<String>,
    field_types: &Vec<&syn::Type>,
    field_attrs: &Vec<FieldAttr>,
    evaluate_descriptor: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let mut field_impls: Vec<proc_macro2::TokenStream> = vec![];
    for ((id, ty), attr) in field_idents.iter().zip(field_types.iter()).zip(field_attrs) {
        let token = match attr.default {
            true => {
                quote! { unwrap_or_default!(#id, #id, #ty) }
            }
            false => {
                quote! { unwrap_or_none!(#id, #id, #ty); }
            }
        };
        field_impls.push(token);
    }

    quote! {
        fn visit_map<_A>(self, mut __map: _A)-> Result<Self::Value, _A::Error>
        where _A: serde_amqp::serde::de::MapAccess<'de>
        {
            #(let mut #field_idents: Option<#field_types> = None;)*

            // The first should always be the descriptor
            let __descriptor: serde_amqp::descriptor::Descriptor = match __map.next_key()? {
                Some(val) => val,
                None => return Err(serde_amqp::serde::de::Error::custom("Expecting__descriptor"))
            };

            #evaluate_descriptor

            while let Some(key) = __map.next_key::<Field>()? {
                match key {
                    #(
                        Field::#field_idents => {
                            if #field_idents.is_some() {
                                return Err(serde_amqp::serde::de::Error::duplicate_field(#field_names))
                            }
                            #field_idents = Some(__map.next_value()?);
                        },
                    )*
                }
            }

            #(
                #field_impls;
            )*
            Ok( #ident{ #(#field_idents, )* } )
        }
    }
}

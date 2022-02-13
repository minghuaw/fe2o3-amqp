use quote::quote;
use syn::{DeriveInput, Fields};

use crate::{
    util::{
        convert_to_case, macro_rules_buffer_if_eq_default, macro_rules_buffer_if_none,
        macro_rules_buffer_if_none_for_tuple_struct, macro_rules_serialize_if_neq_default,
        macro_rules_serialize_if_some, parse_described_struct_attr, parse_named_field_attrs,
    },
    DescribedStructAttr, EncodingType, FieldAttr,
};

pub(crate) fn expand_serialize(
    input: &syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let amqp_attr = parse_described_struct_attr(input);
    let ident = &input.ident;
    match &input.data {
        syn::Data::Struct(data) => expand_serialize_on_datastruct(&amqp_attr, ident, data, input),
        _ => unimplemented!(),
    }
}

fn expand_serialize_on_datastruct(
    amqp_attr: &DescribedStructAttr,
    ident: &syn::Ident,
    data: &syn::DataStruct,
    ctx: &DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let descriptor = match amqp_attr.code {
        Some(code) => quote!(serde_amqp::descriptor::Descriptor::Code(#code)),
        None => {
            let name = &amqp_attr.name[..];
            quote!(serde_amqp::descriptor::Descriptor::Name(serde_amqp::primitives::Symbol::from(#name)))
        }
    };

    match &data.fields {
        Fields::Named(fields) => {
            let token = match fields.named.len() {
                0 => expand_serialize_unit_struct(ident, &descriptor, &amqp_attr.encoding),
                _ => expand_serialize_struct(
                    ident,
                    &descriptor,
                    &amqp_attr.encoding,
                    &amqp_attr.rename_field,
                    fields,
                    ctx,
                ),
            };
            Ok(token)
        }
        Fields::Unnamed(fields) => {
            let token = match fields.unnamed.len() {
                0 => expand_serialize_unit_struct(ident, &descriptor, &amqp_attr.encoding),
                _ => expand_serialize_tuple_struct(ident, &descriptor, &amqp_attr.encoding, fields),
            };
            Ok(token)
        }
        Fields::Unit => Ok(expand_serialize_unit_struct(
            ident,
            &descriptor,
            &amqp_attr.encoding,
        )),
    }
}

fn expand_serialize_unit_struct(
    ident: &syn::Ident,
    descriptor: &proc_macro2::TokenStream,
    encoding: &EncodingType,
) -> proc_macro2::TokenStream {
    let struct_name = match encoding {
        EncodingType::List => quote!(serde_amqp::__constants::DESCRIBED_LIST),
        EncodingType::Basic => panic!("Basic encoding on unit struct is not supported"),
        EncodingType::Map => panic!("Map encoding on unit struct is not supported"),
    };
    quote! {
        #[automatically_derived]
        impl serde_amqp::serde::ser::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde_amqp::serde::ser::Serializer,
            {
                use serde_amqp::serde::ser::SerializeTupleStruct;
                // len + 1 for compatibility with other serializer
                let mut state = serializer.serialize_tuple_struct(#struct_name, 0 + 1)?;
                // serialize descriptor
                state.serialize_field(&#descriptor)?;
                state.end()
            }
        }
    }
}

fn expand_serialize_tuple_struct(
    ident: &syn::Ident,
    descriptor: &proc_macro2::TokenStream,
    encoding: &EncodingType,
    fields: &syn::FieldsUnnamed,
) -> proc_macro2::TokenStream {
    let struct_name = match encoding {
        EncodingType::List => quote!(serde_amqp::__constants::DESCRIBED_LIST),
        EncodingType::Basic => {
            if fields.unnamed.len() == 1 {
                // Basic encoding is allowed on newtype struct
                quote!(serde_amqp::__constants::DESCRIBED_BASIC)
            } else {
                unimplemented!()
            }
        }
        EncodingType::Map => panic!("Map encoding for tuple struct is not supported"),
    };
    let field_indices: Vec<syn::Index> = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, _)| syn::Index::from(i))
        .collect();
    let field_types: Vec<&syn::Type> = fields.unnamed.iter().map(|f| &f.ty).collect();
    let len = field_indices.len();
    let buffer_if_none = macro_rules_buffer_if_none_for_tuple_struct();

    quote! {
        #buffer_if_none

        #[automatically_derived]
        impl serde_amqp::serde::ser::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde_amqp::serde::ser::Serializer,
            {
                use serde_amqp::serde::ser::SerializeTupleStruct;
                let mut null_count = 0u32;
                // len + 1 for compatibility with other serializer
                let mut state = serializer.serialize_tuple_struct(#struct_name, #len + 1)?;
                // serialize descriptor
                // descriptor does not count towards number of element in list
                // in serde_amqp serializer, this will be deducted
                state.serialize_field(&#descriptor)?;
                // #( state.serialize_field(&self.#field_indices)?; )*
                #( buffer_if_none_for_tuple!(state, null_count, &self.#field_indices, #field_types); )*
                state.end()
            }
        }
    }
}

fn expand_serialize_struct(
    ident: &syn::Ident,
    descriptor: &proc_macro2::TokenStream,
    encoding: &EncodingType,
    rename_all: &str,
    fields: &syn::FieldsNamed,
    ctx: &DeriveInput,
) -> proc_macro2::TokenStream {
    let len = fields.named.len();
    let struct_name = match encoding {
        EncodingType::Basic => {
            if fields.named.len() == 1 {
                // Basic encoding is allowed on newtype struct
                quote!(serde_amqp::__constants::DESCRIBED_BASIC)
            } else {
                unimplemented!()
            }
        }
        EncodingType::List => quote!(serde_amqp::__constants::DESCRIBED_LIST),
        EncodingType::Map => quote!(serde_amqp::__constants::DESCRIBED_MAP),
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
    let declarative_macro = match encoding {
        EncodingType::Basic | EncodingType::List => {
            let buffer_if_none = macro_rules_buffer_if_none();

            let buffer_if_eq_default = match field_attrs.contains(&FieldAttr { default: true }) {
                true => macro_rules_buffer_if_eq_default(),
                false => quote! {},
            };
            quote! {
                #buffer_if_none
                #buffer_if_eq_default
            }
        }
        EncodingType::Map => {
            let serialize_if_some = macro_rules_serialize_if_some();
            let serialize_if_neq_default = macro_rules_serialize_if_neq_default();
            quote! {
                #serialize_if_some
                #serialize_if_neq_default
            }
        }
    };

    let mut field_impls: Vec<proc_macro2::TokenStream> = vec![];
    match encoding {
        EncodingType::Basic | EncodingType::List => {
            // for ((id, name), ty) in field_idents
            for (((id, name), ty), attr) in field_idents
                .iter()
                .zip(field_names.iter())
                .zip(field_types.iter())
                .zip(field_attrs.iter())
            {
                let token = match attr.default {
                    true => quote! {
                        buffer_if_eq_default!(state, nulls, &self.#id, #name, #ty);
                    },
                    false => quote! {
                        buffer_if_none!(state, nulls, &self.#id, #name, #ty);
                    },
                };
                field_impls.push(token);
            }
        }
        EncodingType::Map => {
            for (((id, name), ty), attr) in field_idents
                .iter()
                .zip(field_names.iter())
                .zip(field_types.iter())
                .zip(field_attrs.iter())
            {
                let token = match attr.default {
                    true => quote! {
                        serialize_if_neq_default!(state, &self.#id, #name, #ty);
                    },
                    false => quote! {
                        serialize_if_some!(state, &self.#id, #name, #ty);
                    },
                };
                field_impls.push(token);
            }
        }
    }

    quote! {
        #declarative_macro

        #[automatically_derived]
        impl serde_amqp::serde::ser::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde_amqp::serde::ser::Serializer,
            {
                use serde_amqp::serde::ser::SerializeStruct;
                // let mut null_count = 0u32;
                let mut nulls: Vec<&str> = Vec::new();
                // len + 1 for compatibility with other serializer
                let mut state = serializer.serialize_struct(#struct_name, #len + 1)?;
                // serialize descriptor
                // descriptor does not count towards number of element in list
                // in serde_amqp serializer, this will be deducted
                state.serialize_field(serde_amqp::__constants::DESCRIPTOR, &#descriptor)?;
                // #( state.serialize_field(#field_names, &self.#field_idents)?; )*
                // #(buffer_if_none!(state, null_count, &self.#field_idents, #field_names, #field_types);) *
                #( #field_impls; )*
                state.end()
            }
        }
    }
}

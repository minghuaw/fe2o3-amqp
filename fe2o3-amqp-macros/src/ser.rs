use quote::quote;
use syn::{DeriveInput, Fields};

use crate::{
    util::{convert_to_case, parse_described_attr},
    AmqpContractAttr, EncodingType,
};

pub(crate) fn expand_serialize(
    input: &syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let amqp_attr = parse_described_attr(input);
    let ident = &input.ident;
    match &input.data {
        syn::Data::Struct(data) => expand_serialize_on_datastruct(&amqp_attr, ident, data, input),
        _ => unimplemented!(),
    }
}

fn expand_serialize_on_datastruct(
    amqp_attr: &AmqpContractAttr,
    ident: &syn::Ident,
    data: &syn::DataStruct,
    ctx: &DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let descriptor = match amqp_attr.code {
        Some(code) => quote!(fe2o3_amqp::types::Descriptor::Code(#code)),
        None => {
            let name = &amqp_attr.name[..];
            quote!(fe2o3_amqp::types::Descriptor::Name(fe2o3_amqp::types::Symbol::from(#name)))
        }
    };

    match &data.fields {
        Fields::Named(fields) => {
            let token = match fields.named.len() {
                0 => expand_serialize_unit_struct(ident, &descriptor, &amqp_attr.encoding),
                _ => expand_serialize_struct(ident, &descriptor, &amqp_attr.encoding, &amqp_attr.rename_field, fields, ctx)
            };
            Ok(token)
        }
        Fields::Unnamed(fields) => {
            let token = match fields.unnamed.len() {
                0 => expand_serialize_unit_struct(ident, &descriptor, &amqp_attr.encoding),
                _ => expand_serialize_tuple_struct(ident, &descriptor, &amqp_attr.encoding, fields)
            };
            Ok(token)
        }
        Fields::Unit => {
            Ok(expand_serialize_unit_struct(ident, &descriptor, &amqp_attr.encoding))
        }
    }
}

fn expand_serialize_unit_struct(
    ident: &syn::Ident, 
    descriptor: &proc_macro2::TokenStream, 
    encoding: &EncodingType
) -> proc_macro2::TokenStream {
    let struct_name = match encoding {
        EncodingType::List => quote!(fe2o3_amqp::constants::DESCRIBED_LIST),
        EncodingType::Basic => panic!("Basic encoding on unit struct is not supported"),
        EncodingType::Map => panic!("Map encoding on unit struct is not supported"),
    };
    quote! {
        #[automatically_derived]
        impl fe2o3_amqp::serde::ser::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: fe2o3_amqp::serde::ser::Serializer,
            {
                use fe2o3_amqp::serde::ser::SerializeTupleStruct;
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
        EncodingType::List => quote!(fe2o3_amqp::constants::DESCRIBED_LIST),
        EncodingType::Basic => unimplemented!(),
        EncodingType::Map => unimplemented!(),
    };
    let field_indices: Vec<syn::Index> = fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, _)| syn::Index::from(i))
        .collect();
    // .map(|(i,f)| syn::Ident::new(&i.to_string(), f.span())).collect();
    let len = field_indices.len();
    quote! {
        #[automatically_derived]
        impl fe2o3_amqp::serde::ser::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: fe2o3_amqp::serde::ser::Serializer,
            {
                use fe2o3_amqp::serde::ser::SerializeTupleStruct;
                // len + 1 for compatibility with other serializer
                let mut state = serializer.serialize_tuple_struct(#struct_name, #len + 1)?;
                // serialize descriptor
                // descriptor does not count towards number of element in list
                // in fe2o3_amqp serializer, this will be deducted
                state.serialize_field(&#descriptor)?;
                #( state.serialize_field(&self.#field_indices)?; )*
                state.end()
            }
        }
    }
}

fn expand_serialize_struct(
    ident: &syn::Ident, 
    descriptor: &proc_macro2::TokenStream, 
    encoding: &EncodingType,
    rename_field: &str,
    fields: &syn::FieldsNamed,
    ctx: &DeriveInput,
) -> proc_macro2::TokenStream {
    let struct_name = match encoding {
        EncodingType::Basic => quote!(fe2o3_amqp::constants::DESCRIBED_BASIC),
        EncodingType::List => quote!(fe2o3_amqp::constants::DESCRIBED_LIST),
        EncodingType::Map => quote!(fe2o3_amqp::constants::DESCRIBED_MAP),
    };
    let field_idents: Vec<syn::Ident> = fields
        .named
        .iter()
        .map(|f| f.ident.clone().unwrap())
        .collect();
    let field_names: Vec<String> = field_idents
        .iter()
        .map(|i| convert_to_case(rename_field, i.to_string(), ctx).unwrap())
        .collect();
    let len = field_idents.len();

    quote! {
        #[automatically_derived]
        impl fe2o3_amqp::serde::ser::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: fe2o3_amqp::serde::ser::Serializer,
            {
                use fe2o3_amqp::serde::ser::SerializeStruct;
                // len + 1 for compatibility with other serializer
                let mut state = serializer.serialize_struct(#struct_name, #len + 1)?;
                // serialize descriptor
                // descriptor does not count towards number of element in list
                // in fe2o3_amqp serializer, this will be deducted
                state.serialize_field(fe2o3_amqp::constants::DESCRIPTOR, &#descriptor)?;
                #( state.serialize_field(#field_names, &self.#field_idents)?; )*
                state.end()
            }
        }
    }
}
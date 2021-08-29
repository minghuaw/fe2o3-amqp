use quote::quote;
use syn::DeriveInput;

use crate::{AmqpContractAttr, EncodingType, util::{convert_to_case, parse_described_attr}};

pub(crate) fn expand_deserialize(input: &syn::DeriveInput) -> Result<proc_macro2::TokenStream, syn::Error> {
    let attr = parse_described_attr(input);
    let ident = &input.ident;
    match &input.data {
        syn::Data::Struct(data) => {
            expand_deserialize_on_struct(&attr, ident, data, input)
        },
        _ => unimplemented!()
    }
}

fn expand_deserialize_on_struct(
    attr: &AmqpContractAttr, 
    ident: &syn::Ident, 
    data: &syn::DataStruct,
    ctx: &DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let field_idents: Vec<syn::Ident> = data.fields.iter().map(|f| f.ident.clone().unwrap()).collect();
    let field_names: Vec<String> = field_idents.iter()
        .map(|i| 
            convert_to_case(&attr.rename_field, i.to_string(), ctx
        ).unwrap()).collect();    let field_types: Vec<&syn::Type> = data.fields.iter().map(|f| &f.ty).collect();
    let name = &attr.name[..];

    let struct_name = match attr.encoding {
        EncodingType::Basic => quote!(fe2o3_amqp::constants::DESCRIBED_BASIC),
        EncodingType::List => quote!(fe2o3_amqp::constants::DESCRIBED_LIST),
        EncodingType::Map => quote!(fe2o3_amqp::constants::DESCRIBED_MAP)
    };

    let visit_u64 = match attr.code {
        Some(code) => quote! {
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: fe2o3_amqp::serde::de::Error, 
            {
                if v == #code {
                    Ok(Self::Value::descriptor)
                } else {
                    Err(fe2o3_amqp::serde::de::Error::custom("Wrong Descriptor Code"))
                }
            }
        },
        None => quote! {}
    };

    let token = quote! {
        #[automatically_derived]
        impl<'de> fe2o3_amqp::serde::de::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: fe2o3_amqp::serde::de::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                enum Field {
                    descriptor,
                    #(#field_idents, )*
                    // TODO: considering add ignored
                }
                struct FieldVisitor {}
                impl<'de> fe2o3_amqp::serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("field identifier")
                    }

                    // descriptor code is encoded as u64
                    #visit_u64

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: fe2o3_amqp::serde::de::Error, 
                    {
                        match v {
                            #name => Ok(Self::Value::descriptor),
                            #(#field_names => Ok(Self::Value::#field_idents),)*
                            _ => Err(fe2o3_amqp::serde::de::Error::custom("Unknown identifier"))
                        }
                    }

                    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                    where
                        E: fe2o3_amqp::serde::de::Error, 
                    {
                        match v {
                            b if b == #name.as_bytes() => Ok(Self::Value::descriptor),
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

                struct Visitor {}
                impl<'de> fe2o3_amqp::serde::de::Visitor<'de> for Visitor {
                    type Value = #ident;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str(#name)
                    }

                    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: fe2o3_amqp::serde::de::SeqAccess<'de>,
                    {
                        let descriptor: fe2o3_amqp::types::Descriptor = match seq.next_element()? {
                            Some(val) => val,
                            None => return Err(fe2o3_amqp::serde::de::Error::custom("Invalid length"))
                        };

                        #(let #field_idents: #field_types = match seq.next_element()? {
                            Some(val) => val,
                            None => return Err(fe2o3_amqp::serde::de::Error::custom("Invalid length"))
                        };)*

                        Ok( #ident{ #(#field_idents, )* } )
                    }
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
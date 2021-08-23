use darling::{FromDeriveInput, FromMeta};
use quote::quote;
use syn::spanned::Spanned;

#[derive(Debug, Clone, FromMeta)]
#[darling(default)]
enum EncodingType {
    Basic,
    List,
    Map,
}

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(amqp_contract))]
struct AmqpContractAttr {
    #[darling(default)]
    pub name: Option<String>,
    #[darling(default)]
    pub code: Option<u64>,
    #[darling(default)]
    pub encoding: Option<EncodingType>,
}

#[proc_macro_derive(AmqpContract, attributes(amqp_contract))]
pub fn derive_amqp_contract(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let ident = &input.ident;
    let ident_str = ident.to_string();
    let attr = AmqpContractAttr::from_derive_input(&input).unwrap();
    println!("{:?}", &attr);
    
    let descriptor = match attr.code {
        Some(code) => {
            quote! {
                fe2o3_amqp::descriptor::Descriptor::Code(#code)
            }
        },
        None => {
            match attr.name {
                Some(name) => {
                    quote! {
                        fe2o3_amqp::descriptor::Descriptor::Name(fe2o3_amqp::types::Symbol::from(#name.to_string()))
                    }
                },
                None => {
                    quote! {
                        fe2o3_amqp::descriptor::Descriptor::Name(fe2o3_amqp::types::Symbol::from(#ident_str))
                    }
                }
            }
        }
    };

    let encoding = match attr.encoding {
        Some(enc) => {
            match enc {
                EncodingType::Basic => quote!{ fe2o3_amqp::described::EncodingType::Basic },
                EncodingType::List => quote!{ fe2o3_amqp::described::EncodingType::List },
                EncodingType::Map => quote!{ fe2o3_amqp::described::EncodingType::Map }
            }
        },
        None => {
            match input.data {
                syn::Data::Struct(s) => {
                    match &s.fields {
                        syn::Fields::Named(_) => {
                            quote! { fe2o3_amqp::described::EncodingType::List }
                        },
                        syn::Fields::Unnamed(unnamed) => {
                            match s.fields.len() {
                                0 => {
                                    return Err(syn::Error::new(unnamed.span(), "At least one field should be present"))
                                        .unwrap_or_else(|err| err.to_compile_error())
                                        .into()
                                },
                                1 => {
                                    quote! { fe2o3_amqp::described::EncodingType::Basic }
                                },
                                _ => {
                                    quote! { fe2o3_amqp::described::EncodingType::List }
                                } 
                            }
                        },
                        syn::Fields::Unit => {
                            quote! { fe2o3_amqp::described::EncodingType::Basic }
                        }
                    }
                },
                syn::Data::Enum(e) => {
                    return Err(syn::Error::new(e.enum_token.span, "Enum not implemented"))
                        .unwrap_or_else(|err| err.to_compile_error())
                        .into();
                },
                syn::Data::Union(u) => {
                    return Err(syn::Error::new(u.union_token.span, "Union not implemented"))
                        .unwrap_or_else(|err| err.to_compile_error())
                        .into();
                }
            }
        }
    };

    let impl_try_from = quote!{
        impl std::convert::TryFrom<#ident> for fe2o3_amqp::described::Described<#ident> {
            type Error = fe2o3_amqp::error::Error;

            fn try_from(value: #ident) -> Result<fe2o3_amqp::described::Described<#ident>, Self::Error> {
                Ok(
                    fe2o3_amqp::described::Described::new(
                        #encoding,
                        #descriptor,
                        value
                    )
                )
            }
        }
    };

    // let try_from_impl = match input.data {
    //     syn::Data::Struct(s) => {
    //         match &s.fields {
    //             syn::Fields::Named(named) => {
                    
    //             },
    //             syn::Fields::Unnamed(unnamed) => {
    //                 match s.fields.len() {
    //                     0 => {
    //                         return Err(syn::Error::new(unnamed.span(), "At least one field should be present"))
    //                             .unwrap_or_else(|err| err.to_compile_error())
    //                             .into()
    //                     },
    //                     1 => {
    //                         quote!{
    //                             impl std::convert::TryFrom<#ident> for fe2o3_amqp::described::Described<#ident> {
    //                                 type Error = fe2o3_amqp::error::Error;

    //                                 fn try_from(value: #ident) -> Result<fe2o3_amqp::described::Described<#ident>, Self::Error> {
    //                                     Ok(
    //                                         fe2o3_amqp::described::Described::new(
    //                                             fe2o3_amqp::described::EncodingType::Basic,
    //                                             #descriptor,
    //                                             value
    //                                         )
    //                                     )
    //                                 }
    //                             }
    //                         }
    //                     },
    //                     _ => {
    //                         quote!{
    //                             impl std::convert::TryFrom<#ident> for fe2o3_amqp::described::Described<#ident> {
    //                                 type Error = fe2o3_amqp::error::Error;

    //                                 fn try_from(value: #ident) -> Result<fe2o3_amqp::described::Described<#ident>, Self::Error> {
    //                                     Ok(
    //                                         fe2o3_amqp::described::Described::new(
    //                                             fe2o3_amqp::described::EncodingType::List,
    //                                             #descriptor,
    //                                             value
    //                                         )
    //                                     )
    //                                 }
    //                             }
    //                         }
    //                     } 
    //                 }
    //             },
    //             syn::Fields::Unit => {
    //                 quote!{
    //                     impl std::convert::TryFrom<#ident> for fe2o3_amqp::described::Described<#ident> {
    //                         type Error = fe2o3_amqp::error::Error;

    //                         fn try_from(value: #ident) -> Result<fe2o3_amqp::described::Described<#ident>, Self::Error> {
    //                             Ok(
    //                                 fe2o3_amqp::described::Described::new(
    //                                     fe2o3_amqp::described::EncodingType::List,
    //                                     #descriptor,
    //                                     value
    //                                 )
    //                             )
    //                         }
    //                     }
    //                 }
    //             }
    //         }
    //     },
    //     syn::Data::Enum(e) => {
    //         return Err(syn::Error::new(e.enum_token.span, "Enum not implemented"))
    //             .unwrap_or_else(|err| err.to_compile_error())
    //             .into();
    //     },
    //     syn::Data::Union(u) => {
    //         return Err(syn::Error::new(u.union_token.span, "Union not implemented"))
    //             .unwrap_or_else(|err| err.to_compile_error())
    //             .into();
    //     }
    // };

    let output = quote::quote! {
        #impl_try_from
    };
    output.into()
}

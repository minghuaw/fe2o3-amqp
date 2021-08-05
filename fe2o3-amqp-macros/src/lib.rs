use darling::{FromDeriveInput, FromMeta};

#[derive(Debug, Clone, FromMeta)]
#[darling(default)]
enum EncodingType {
    List,
    Map
}

impl Default for EncodingType {
    fn default() -> Self {
        Self::List
    }
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
    #[darling(default)]
    pub described: bool,
}

#[proc_macro_derive(AmqpContract, attributes(amqp_contract))]
pub fn derive_amqp_contract(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    unimplemented!()
    // let input = syn::parse_macro_input!(item as syn::DeriveInput);
    // let ident = &input.ident;
    // let attr = AmqpContractAttr::from_derive_input(&input).unwrap();
    // println!("{:?}", &attr);
    // let fn_get_name = match attr.name {
    //     Some(s) => {
    //         quote::quote! {
    //             fn get_name() -> Option<String> { Some(#s.into()) }
    //         }
    //     },
    //     None => {
    //         quote::quote! {
    //             fn get_name() -> Option<String> { None }
    //         }
    //     }
    // };
    // let fn_get_code = match attr.code {
    //     Some(num) => {
    //         quote::quote! {
    //             fn get_code() -> Option<u64> { Some(#num) }
    //         }
    //     },
    //     None => {
    //         quote::quote! {
    //             fn get_code() -> Option<u64> { None }
    //         }
    //     }
    // };
    // let fn_get_encoding = match attr.encoding {
    //     Some(s) => {
    //         match s {
    //             EncodingType::List => {
    //                 quote::quote! {
    //                     fn get_encoding_type() -> Option<fe2o3_amqp::contract::EncodingType>{ 
    //                         Some(fe2o3_amqp::contract::EncodingType::List)
    //                     }
    //                 }
    //             },
    //             EncodingType::Map => {
    //                 quote::quote! {
    //                     fn get_encoding_type() -> Option<fe2o3_amqp::contract::EncodingType>{ 
    //                         Some(fe2o3_amqp::contract::EncodingType::Map)
    //                     }
    //                 }
    //             }
    //         }
    //     },
    //     None => {
    //         quote::quote! {
    //             fn get_encoding_type() -> Option<fe2o3_amqp::contract::EncodingType>{ None }
    //         }
    //     }
    // };
    // let is_described = attr.described;
    // let fn_is_described = quote::quote! {
    //     fn is_described() -> bool { #is_described }
    // };

    // let output = quote::quote! { 
    //     impl fe2o3_amqp::contract::AmqpContract for #ident {
    //         #fn_get_name
    //         #fn_get_code
    //         #fn_get_encoding
    //         #fn_is_described
    //     }
    // };
    // output.into()
}


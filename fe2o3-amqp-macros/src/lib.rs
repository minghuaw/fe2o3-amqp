use darling::{FromDeriveInput, FromMeta};

#[derive(Debug, Clone, FromMeta)]
#[darling(default)]
enum EncodingType {
    List,
    Map,
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
}

#[proc_macro_derive(AmqpContract, attributes(amqp_contract))]
pub fn derive_amqp_contract(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let ident = &input.ident;
    let attr = AmqpContractAttr::from_derive_input(&input).unwrap();
    println!("{:?}", &attr);

    let output = quote::quote! {
        impl fe2o3_amqp::contract::AmqpContract for #ident {

        }
    };
    output.into()
}

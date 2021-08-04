use darling::{FromDeriveInput, FromMeta};

#[derive(Debug, Clone, FromMeta)]
enum EncodingType {
    Seq,
    Map
}

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(amqp_contract))]
struct AmqpContractAttr {
    #[darling(default)]
    name: Option<String>,
    #[darling(default)]
    code: Option<u64>,
    #[darling(default)]
    encoding_type: Option<EncodingType>,
}

#[proc_macro_derive(AmqpContract, attributes(amqp_contract))]
pub fn derive_amqp_contract(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr = AmqpContractAttr::from_derive_input(&input);
    println!("{:?}", &attr);
    let output = quote::quote! { };
    output.into()
}
use darling::FromDeriveInput;

use crate::{AmqpContractAttr, DescribedAttr, EncodingType};

pub(crate) fn parse_described_attr(input: &syn::DeriveInput) -> AmqpContractAttr {
    let attr = DescribedAttr::from_derive_input(&input).unwrap();

    let name = attr.name.unwrap_or_else(|| input.ident.to_string());
    let code = attr.code;
    let encoding = attr.encoding.unwrap_or(EncodingType::List);
    AmqpContractAttr { name, code, encoding }
}
// use fe2o3_amqp::macros::{Described, NonDescribed};

// #[derive(Debug, Described)]
// #[amqp_contract(name = "a", code = 0x18, encoding = "list")]
// struct Test {
//     a: i32,
//     b: bool,
// }

// #[derive(Debug, Described)]
// struct NewType(i32);

// #[derive(Debug, NonDescribed)]
// struct Foo {
//     is_not_described: bool,
// }

// #[cfg(test)]
// mod tests {
//     use std::convert::TryInto;

//     use fe2o3_amqp::types::Described;

//     use super::*;

//     #[test]
//     fn test_try_into_described() {
//         let test = Test { a: 1, b: true };
//         let new_type = NewType(13);
//         let foo = Foo { is_not_described: true };

//         let test_try_into: Result<Described<_>, _> = test.try_into();
//         let new_type_try_into: Result<Described<_>, _> = new_type.try_into();
//         let foo_try_into: Result<Described<_>, _> = foo.try_into();
//         assert!(test_try_into.is_ok());
//         assert!(new_type_try_into.is_ok());
//         assert!(foo_try_into.is_err());
//     }
// }

use fe2o3_amqp::macros::{DeserializeComposite, SerializeComposite};

// macro_rules! buffer_if_none {
//     ($state: ident, $field_ident: expr, $field_name: expr, Option<$ftype: ty>) => {
//         // if self.field_ident.is_some() {
//         //     for _ in 0..null_count {
//         //         // name is not used in list encoding
//         //         state.serialize_field("", &())?; // None and () share the same encoding
//         //     }
//         //     null_count = 0;
//             $state.serialize_field($field_name, $field_ident)?;
//         // } else {
//         //     null_count += 1;
//         // }
//     };
//     ($state: ident, $field_ident: expr, $field_name: expr, $ftype: ty) => {
//         // for _ in 0..null_count {
//         //     // name is not used in list encoding
//         //     state.serialize_field("", &())?; // None and () share the same encoding
//         // }
//         // null_count = 0;
//         $state.serialize_field($field_name, $field_ident)?;
//     };
// }

#[derive(SerializeComposite, DeserializeComposite)]
#[amqp_contract(code = 0x13, encoding = "list")]
struct Foo {
    is_fool: Option<bool>,
    a: Option<i32>,
}

#[derive(SerializeComposite, DeserializeComposite)]
#[amqp_contract(encoding="list")]
struct Unit { }

#[derive(SerializeComposite, DeserializeComposite)]
struct TupleStruct(Option<i32>, bool);

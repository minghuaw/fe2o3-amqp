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

use fe2o3_amqp::macros::SerializeDescribed;

#[derive(SerializeDescribed)]
#[amqp_contract(name="bar", encoding="map")]
struct Foo {
    is_fool: bool,
    second_field: String,
}
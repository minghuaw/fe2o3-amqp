//! Custom derive macros. Requires feature `"derive"`.

pub use serde_amqp_derive::{DeserializeComposite, SerializeComposite};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ser::to_vec;

    use crate as serde_amqp;

    #[test]
    fn test_macro_integration() {
        #[derive(Debug, PartialEq, SerializeComposite, DeserializeComposite)]
        #[amqp_contract(name = "a", encoding = "list")]
        struct Test {
            a: i32,
            b: bool,
        }

        let value = Test { a: 7, b: true };
        let buf = to_vec(&value).unwrap();
        let deserialized: Test = crate::de::from_slice(&buf).unwrap();
        assert_eq!(deserialized, value);
    }
}

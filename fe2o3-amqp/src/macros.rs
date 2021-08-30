pub use fe2o3_amqp_macros::{DeserializeComposite, SerializeComposite};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ser::to_vec;

    use crate as fe2o3_amqp;

    #[test]
    fn test_macro_integration() {
        #[derive(Debug, SerializeComposite, DeserializeComposite)]
        #[amqp_contract(name = "a", encoding = "list")]
        struct Test {
            a: i32,
            b: bool,
        }

        let value = Test { a: 7, b: true };
        let buf = to_vec(&value).unwrap();
        println!("{:x?}", buf);
    }
}

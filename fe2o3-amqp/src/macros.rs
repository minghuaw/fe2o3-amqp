pub use fe2o3_amqp_macros::{Described, NonDescribed, SerializeDescribed,};

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use crate::ser::{to_vec, to_vec_described, serialize};
    use crate::convert::{IntoDescribed};

    use super::*;

    use crate as fe2o3_amqp;

    #[test]
    fn test_macro_integration() {
        #[derive(Debug, Serialize, Deserialize, Described)]
        #[amqp_contract(name = "a", code = 0x8, encoding = "list")]
        struct Test {
            a: i32,
            b: bool,
        }

        let value = Test {a: 7, b: true};
        let buf = to_vec(&value).unwrap();
        println!("{:?}", buf);

        let value = Test {a: 7, b: true};
        let buf = to_vec(&value.into_described()).unwrap();
        println!("{:?}", buf);

        let value = Test {a: 7, b: true};
        let buf = to_vec_described(&value.into_described()).unwrap();
        println!("{:?}", buf);

        let value = Test {a: 7, b: true};
        let buf = serialize(&value).unwrap();
        println!("{:?}", buf);
    }

}
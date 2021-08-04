pub use fe2o3_amqp_macros::AmqpContract;

#[cfg(test)]
mod test {
    #[test]
    fn test_amqp_contract_macro_expand() {
        use super::AmqpContract;
        // use serde::{Serialize, Deserialize};

        #[derive(Debug, AmqpContract)]
        struct Test {
            a: i32,
            b: bool
        }
    }
}
use serde::{Serialize, Deserialize};
use serde_amqp::{SerializeComposite, DeserializeComposite, to_vec};


#[derive(Debug, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "test:example:*",
    code = 0x0000_0000_0001_0001,
    encoding = "list",
)]
struct TestExample<T> {
    a: T
}

#[test]
fn single_bool() {
    let value = TestExample {a: true};
    let buf = to_vec(&value).unwrap();
    println!("{:#x?}", buf);
}
use serde_amqp::{to_vec, DeserializeComposite, SerializeComposite};

#[derive(Debug, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "test:example:*",
    code = "0x0000_0001:0000_0001",
    encoding = "list"
)]
struct TestExample<T> {
    a: T,
}

#[test]
fn single_bool() {
    let value = TestExample { a: true };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x2, 0x1, 0x41,
    ];
    assert_eq!(buf, expected);
}

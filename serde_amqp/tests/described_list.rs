use serde_amqp::{to_vec, DeserializeComposite, SerializeComposite, from_slice};

#[derive(Debug, SerializeComposite, DeserializeComposite, PartialEq, PartialOrd)]
#[amqp_contract(
    name = "test:example:*",
    code = "0x0000_0001:0000_0001",
    encoding = "list"
)]
struct Single<T> {
    a: T,
}

#[test]
fn single_bool() {
    let value = Single { a: true };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x2, 0x1, 0x41,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<bool> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[test]
fn single_i8() {
    let value = Single { a: 0i8 };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x3, 0x1, 0x51, 0x00,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i8> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

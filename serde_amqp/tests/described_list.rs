use serde::{Deserialize, Serialize};
#[cfg(feature = "derive")]
use serde_amqp::{from_slice, to_vec, DeserializeComposite, SerializeComposite};

#[cfg(feature = "derive")]
#[derive(Debug, SerializeComposite, DeserializeComposite, PartialEq, PartialOrd)]
#[amqp_contract(
    name = "test:example:*",
    code = "0x0000_0001:0000_0001",
    encoding = "list"
)]
struct Single<T> {
    a: T,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
struct CustomStruct {
    a: u32,
    b: i32,
    c: bool,
    d: String,
}

#[cfg(feature = "derive")]
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

#[cfg(feature = "derive")]
#[test]
fn single_u8_min() {
    let value = Single { a: u8::MIN };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x3, 0x1, 0x50, 0x00,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<u8> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_u8_max() {
    let value = Single { a: u8::MAX };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x3, 0x1, 0x50, 0xff,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<u8> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_u16_min() {
    let value = Single { a: u16::MIN };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x4, 0x1, 0x60, 0x00, 0x00,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<u16> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_u16_max() {
    let value = Single { a: u16::MAX };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x4, 0x1, 0x60, 0xff, 0xff,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<u16> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_u32_min() {
    let value = Single { a: u32::MIN };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x2, 0x1, 0x43,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<u32> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_u32_max() {
    let value = Single { a: u32::MAX };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x6, 0x1, 0x70, 0xff, 0xff, 0xff,
        0xff,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<u32> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_u64_min() {
    let value = Single { a: u64::MIN };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x2, 0x1, 0x44,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<u64> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_u64_max() {
    let value = Single { a: u64::MAX };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0xa, 0x1, 0x80, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<u64> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_i8_zero() {
    let value = Single { a: 0i8 };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x3, 0x1, 0x51, 0x00,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i8> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_i8_negative_one() {
    let value = Single { a: -1i8 };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x3, 0x1, 0x51, 0xff,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i8> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_i8_max() {
    let value = Single { a: i8::MAX };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x3, 0x1, 0x51, 0x7f,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i8> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_i8_min() {
    let value = Single { a: i8::MIN };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x3, 0x1, 0x51, 0x80,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i8> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_i16() {
    let value = Single { a: 0i16 };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x4, 0x1, 0x61, 0x00, 0x00,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i16> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_i32_zero() {
    let value = Single { a: 0i32 };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x3, 0x1, 0x54, 0x00,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i32> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_i32_negative_one() {
    let value = Single { a: -1i32 };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x3, 0x1, 0x54, 0xff,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i32> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_i32_max() {
    let value = Single { a: i32::MAX };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x6, 0x1, 0x71, 0x7f, 0xff, 0xff,
        0xff,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i32> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_i32_min() {
    let value = Single { a: i32::MIN };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x6, 0x1, 0x71, 0x80, 0x00, 0x00,
        0x00,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i32> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_i64_zero() {
    let value = Single { a: 0i64 };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x3, 0x1, 0x55, 0x00,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i64> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_i64_negative_one() {
    let value = Single { a: -1i64 };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x3, 0x1, 0x55, 0xff,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i64> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_i64_max() {
    let value = Single { a: i64::MAX };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0xa, 0x1, 0x81, 0x7f, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i64> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_i64_min() {
    let value = Single { a: i64::MIN };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0xa, 0x1, 0x81, 0x80, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<i64> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_str8() {
    let value = Single { a: "hello world" };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0xe, 0x1, 0xa1, 0x0b, 0x68, 0x65,
        0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<&str> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_string8() {
    let value = Single {
        a: String::from("hello world"),
    };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0xe, 0x1, 0xa1, 0x0b, 0x68, 0x65,
        0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<String> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_sym8() {
    use serde_amqp::primitives::Symbol;

    let value = Single {
        a: Symbol::new("hello world"),
    };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0xe, 0x1, 0xa3, 0x0b, 0x68, 0x65,
        0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<Symbol> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_list0() {
    let value = Single {
        a: Vec::<u8>::new(),
    };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x2, 0x1, 0x45,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<Vec<u8>> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_list8() {
    let value = Single {
        a: vec![1, 2, 3, 4, 5],
    };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0xe, 0x1, 0xc0, 0xb, 0x5, 0x50,
        0x01, 0x50, 0x02, 0x50, 0x03, 0x50, 0x04, 0x50, 0x05,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<Vec<u8>> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_map8() {
    use std::collections::BTreeMap;

    let mut map = BTreeMap::new();
    map.insert("key", "value");
    let value = Single { a: map };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x10, 0x1, 0xc1, 0xd, 0x2, 0xa1,
        0x03, 0x6b, 0x65, 0x79, 0xa1, 0x05, 0x76, 0x61, 0x6c, 0x75, 0x65,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<BTreeMap<&str, &str>> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_array8() {
    use serde_amqp::primitives::Array;

    let array = Array::from(vec![1, 2, 3, 4, 5]);
    let value = Single { a: array };
    let buf = to_vec(&value).unwrap();
    // There is not optimization for arrays, so i32 is always encoded as 4 bytes
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x19, 0x1, 0xe0, 0x16, 0x5, 0x71,
        0x0, 0x0, 0x0, 0x01, 0x0, 0x0, 0x0, 0x02, 0x0, 0x0, 0x0, 0x03, 0x0, 0x0, 0x0, 0x04, 0x0,
        0x0, 0x0, 0x05,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<Array<i32>> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

#[cfg(feature = "derive")]
#[test]
fn single_custom_struct() {
    let value = Single {
        a: CustomStruct {
            a: 1,
            b: 2,
            c: false,
            d: "hello world".to_string(),
        },
    };
    let buf = to_vec(&value).unwrap();
    let expected = [
        0x0, 0x80, 0x0, 0x0, 0x0, 0x1, 0x0, 0x0, 0x0, 0x1, 0xc0, 0x16, 0x1, 0xc0, 0x13, 0x4, 0x52,
        0x1, 0x54, 0x2, 0x42, 0xa1, 0xb, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72,
        0x6c, 0x64,
    ];
    assert_eq!(buf, expected);

    let decoded: Single<CustomStruct> = from_slice(&buf).unwrap();
    assert_eq!(decoded, value);
}

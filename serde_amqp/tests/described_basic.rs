use serde::{Deserialize, Serialize};

#[cfg(feature = "derive")]
use serde_amqp::{DeserializeComposite, SerializeComposite};

#[cfg(feature = "derive")]
#[derive(Debug, Clone, PartialEq, Eq, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "test:example:*",
    code = "0x0000_462c:0x0000_0011",
    encoding = "basic"
)]
struct Single<T>(T);

#[derive(Debug, Serialize, Deserialize)]
struct CustomStruct {
    a: u8,
    b: u16,
}

#[cfg(feature = "derive")]
#[test]
fn single_bool() {
    let value = Single(true);
    let encoded = serde_amqp::to_vec(&value).unwrap();
    let expected = [
        0x00, 0x80, 0x00, 0x00, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x11, 0x41,
    ];
    assert_eq!(encoded, expected);
    let decoded = serde_amqp::from_slice::<Single<bool>>(&encoded).unwrap();
    assert_eq!(value, decoded);
}

#[cfg(feature = "derive")]
#[test]
fn single_u8() {
    let value = Single(42u8);
    let encoded = serde_amqp::to_vec(&value).unwrap();
    let expected = [
        0x00, 0x80, 0x00, 0x00, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x11, 0x50, 0x2a,
    ];
    assert_eq!(encoded, expected);
    let decoded = serde_amqp::from_slice::<Single<u8>>(&encoded).unwrap();
    assert_eq!(value, decoded);
}

#[cfg(feature = "derive")]
#[test]
fn single_u16() {
    let value = Single(42u16);
    let encoded = serde_amqp::to_vec(&value).unwrap();
    let expected = [
        0x00, 0x80, 0x00, 0x00, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x11, 0x60, 0x00, 0x2a,
    ];
    assert_eq!(encoded, expected);
    let decoded = serde_amqp::from_slice::<Single<u16>>(&encoded).unwrap();
    assert_eq!(value, decoded);
}

#[cfg(feature = "derive")]
#[test]
fn single_u32() {
    let value = Single(42u32);
    let encoded = serde_amqp::to_vec(&value).unwrap();
    let expected = [
        0x00, 0x80, 0x00, 0x00, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x11, 0x52, 0x2a,
    ];
    assert_eq!(encoded, expected);
    let decoded = serde_amqp::from_slice::<Single<u32>>(&encoded).unwrap();
    assert_eq!(value, decoded);
}

#[cfg(feature = "derive")]
#[test]
fn single_u64() {
    let value = Single(300u64);
    let encoded = serde_amqp::to_vec(&value).unwrap();
    let expected = [
        0x00, 0x80, 0x00, 0x00, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x11, 0x80, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x01, 0x2c,
    ];
    assert_eq!(encoded, expected);
    let decoded = serde_amqp::from_slice::<Single<u64>>(&encoded).unwrap();
    assert_eq!(value, decoded);
}

#[cfg(feature = "derive")]
#[test]
fn single_i8() {
    let value = Single(42i8);
    let encoded = serde_amqp::to_vec(&value).unwrap();
    let expected = [
        0x00, 0x80, 0x00, 0x00, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x11, 0x51, 0x2a,
    ];
    assert_eq!(encoded, expected);
    let decoded = serde_amqp::from_slice::<Single<i8>>(&encoded).unwrap();
    assert_eq!(value, decoded);
}

#[cfg(feature = "derive")]
#[test]
fn single_i16() {
    let value = Single(42i16);
    let encoded = serde_amqp::to_vec(&value).unwrap();
    let expected = [
        0x00, 0x80, 0x00, 0x00, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x11, 0x61, 0x00, 0x2a,
    ];
    assert_eq!(encoded, expected);
    let decoded = serde_amqp::from_slice::<Single<i16>>(&encoded).unwrap();
    assert_eq!(value, decoded);
}

#[cfg(feature = "derive")]
#[test]
fn single_i32() {
    let value = Single(42i32);
    let encoded = serde_amqp::to_vec(&value).unwrap();
    let expected = [
        0x00, 0x80, 0x00, 0x00, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x11, 0x54, 0x2a,
    ];
    assert_eq!(encoded, expected);
    let decoded = serde_amqp::from_slice::<Single<i32>>(&encoded).unwrap();
    assert_eq!(value, decoded);
}

#[cfg(feature = "derive")]
#[test]
fn single_i64() {
    let value = Single(15i64);
    let encoded = serde_amqp::to_vec(&value).unwrap();
    let expected = [
        0x00, 0x80, 0x00, 0x00, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x11, 0x55, 0x0f,
    ];
    assert_eq!(encoded, expected);
    let decoded = serde_amqp::from_slice::<Single<i64>>(&encoded).unwrap();
    assert_eq!(value, decoded);
}

#[cfg(feature = "derive")]
#[test]
fn single_str8() {
    let value = Single("hello");
    let encoded = serde_amqp::to_vec(&value).unwrap();
    let expected = [
        0x00, 0x80, 0x00, 0x00, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x11, 0xa1, 0x05, 0x68, 0x65, 0x6c,
        0x6c, 0x6f,
    ];
    assert_eq!(encoded, expected);
    let decoded = serde_amqp::from_slice::<Single<&str>>(&encoded).unwrap();
    assert_eq!(value, decoded);
}

#[cfg(feature = "derive")]
#[test]
fn single_list8() {
    let value = Single(vec![1u8, 2, 3]);
    let encoded = serde_amqp::to_vec(&value).unwrap();
    let expected = [
        0x00, 0x80, 0x00, 0x00, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x11, 0xc0, 0x07, 0x03, 0x50, 0x01,
        0x50, 0x02, 0x50, 0x03,
    ];
    assert_eq!(encoded, expected);
    let decoded = serde_amqp::from_slice::<Single<Vec<u8>>>(&encoded).unwrap();
    assert_eq!(value, decoded);
}

#[cfg(feature = "derive")]
#[test]
fn single_map8() {
    use serde_amqp::primitives::OrderedMap;

    let mut map = OrderedMap::new();
    map.insert("a", 1u8);
    map.insert("b", 2u8);
    map.insert("c", 3u8);
    let value = Single(map);
    let encoded = serde_amqp::to_vec(&value).unwrap();
    let expected = [
        0x00, 0x80, 0x00, 0x00, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x11, 0xc1, 0x10, 0x06, 0xa1, 0x01,
        0x61, 0x50, 0x01, 0xa1, 0x01, 0x62, 0x50, 0x02, 0xa1, 0x01, 0x63, 0x50, 0x03,
    ];
    assert_eq!(encoded, expected);
    let decoded = serde_amqp::from_slice::<Single<OrderedMap<&str, u8>>>(&encoded).unwrap();
    assert_eq!(value, decoded);
}

#[cfg(feature = "derive")]
#[test]
fn single_array8() {
    use serde_amqp::primitives::Array;

    let array = Array::from(vec![1i32, 2, 3]);
    let value = Single(array);
    let encoded = serde_amqp::to_vec(&value).unwrap();
    let expected = [
        0x00, 0x80, 0x00, 0x00, 0x46, 0x2c, 0x00, 0x00, 0x00, 0x11, 0xe0, 0x0e, 0x03, 0x71, 0x00,
        0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03,
    ];
    assert_eq!(encoded, expected);
    let decoded = serde_amqp::from_slice::<Single<Array<i32>>>(&encoded).unwrap();
    assert_eq!(value, decoded);
}

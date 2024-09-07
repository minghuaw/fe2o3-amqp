# Change Log

## 0.12.3

1. Fix warning caused by [changes to never type fallback for edition
   2024](https://github.com/rust-lang/rust/issues/123748)

## 0.12.2

1. Fixed bug caused by not creating new `SizeSerializer` in `SerializeSeq::serialize_element` (issue #279)

## 0.12.1

1. Fixed empty `Array` not deserialized properly (issue #277)

## 0.12.0

### Breaking

1. Changed return types of functions in `serde_amqp::read::Read` trait
2. Drain `IoReader` and `SliceReader` even when `read_exact` fails

## 0.11.1

1. Fixed a bug with value serializer that doesn't handle nested composite type properly

## 0.11.0

1. Removed deprecated `remove` and `remove_entry` in `OrderedMap`

## 0.10.0

1. Unified versioning with other `fe2o3-amqp` crates

## 0.5.10

1. Backported `0.9.1`

## 0.9.1

1. Mark `remove` and `remove_entry` as deprecated in `OrderedMap`
2. Added `swap_remove` and `swap_remove_entry` in `OrderedMap`
3. Added `shift_remove` and `shift_remove_entry` in `OrderedMap`

## 0.9.0

1. Unified versioning with other `fe2o3-amqp` crates
2. Changed `Value` variants to strictly follow the rust naming convention
3. Updated `ordered-float` to "4"
4. Updated `indexmap` to "2"

## 0.5.9

1. Fixed a bug where `str` and `bytes` of length `u8::MAX` are serialized incorrectly as `str8-utf8`
   and `vbin8`, respectively. After the fix, `str` and `bytes` with length less than or equal to
   `u8::MAX - 1` are serialized as `str8-utf8` and `vbin8`, respectively. `str` and `bytes` with
   length greater than or equal to `u8::MAX` and less than `u32::MAX - 4` are serialized as
   `str32-utf8` and `vbin32`, respectively.

## 0.5.8

1. Fixed clippy warnings

## 0.5.7

1. Changed `From<Described<Value>> for Value` to `From<Described<T>> for Value where T: Into<Value>`

## 0.5.6

1. Added `Borrow<str>` impl for `&'a SymbolRef<'b>`

## 0.5.5

1. Changed conversion from `Timestamp` to `time::OffsetDateTime` to infallible

## 0.5.4

1. Updated macro to "0.2.1" to allow using raw u64 as the descriptor code (ie. `0x0000_0000_0000_0000`)

## 0.5.3

1. Added "chrono-preview" feature to reflect deprecation of `from_timestamp` in `chrono` crate.

## 0.5.2

1. Fixed a bug where variable length types like str or vbin doesn't handle boundary condition at u8::MAX
2. Added `serialized_size` method that returns the size of the serialized data without actually serializing it to a buffer

## 0.5.1

1. Added feature gated conversion impls for `Timestamp` from/to
   1. `chrono::DateTime`
   2. `chrono::Duration`
   3. `time::OffsetDateTime`
   4. `time::Duration`
2. Added feature gated conversion impls for `Uuid` from/to
   1. `uuid::Uuid`

## 0.5.0

1. Fixed [#117](https://github.com/minghuaw/fe2o3-amqp/issues/117) with `serde_amqp_derive = "0.2.0"`
2. Added `extensions::TransparentVec` type that is feature gated behind `"extensions"` feature flag.

## 0.4.5

1. Fixed a bug with serializing/deserilizing described basic wrapper that cause a bug on `AmqpValue` wrapping around a custom struct

## 0.4.4

1. Impl `IntoIterator` for `Array<T>`
2. Impl `TryFrom<Value>` for `Vec<T>`, `Array<T>`, and `OrderedMap<K,V>`

## 0.4.3

1. Changed `Read` trait to reflect `UnexpectedEof` as a `None`

## 0.4.2

1. Added `BinaryRef<'a>` which allows serialize/deserialize as bytes on the wrapped `&[u8]`, and
   implements `LowerHex` and `UpperHex` formatting.
2. Added `LowerHex` and `UpperHex` formatting for `serde_amqp::primitives::Uuid`

## 0.4.1

1. Added `impl<'a> From<SymbolRef<'a>> for Symbol`

## 0.4.0

1. Bug fixes (Breaking)
   1. Switched from `Value::Map(BTreeMap<Value, Value>)` to `Value::Map(OrderedMap<Value, Value>)` to preserve the encoded order
2. New features
   1. Added `OrderedMap` which is a wrapper around `IndexMap` with custom implementation of some traits
   2. More `TryFrom<Value>` impl for common map types

## 0.3.1

1. Made constant `VALUE` public for possible downstream uses

## 0.3.0

1. Breaking Changes
   1. For consistencies, changed:
      1. `EncodingCodes::Ulong0` to `EncodingCodes::Ulong0`
      2. `EncodingCodes::SmallUlong` to `EncodingCodes::SmallUlong`
      3. `EncodingCodes::Uint0` to `EncodingCodes::Uint0`
      4. `EncodingCodes::SmallUint` to `EncodingCodes::SmallUint`

## 0.2.5

1. Added `impl From<Array/Vec/BTreeMap> for Value`
2. Fixed cannot deserialize `Array<Value>` with `serde_json`

## 0.2.4

1. Fixed typos in Readme

## 0.2.3

1. Added `Borrow<str>` impl for `Symbol` and `SymbolRef`

## 0.2.2

1. Added `SymbolRef` which takes a `&str` instead of `String`
2. Added impl of `TryFrom` for `Value` variant types

## 0.2.1

1. Instead of clearing `IoReader` buffer, only drain the `len` needed.

## 0.2.0

1. Breaking bug fixes
   1. `Array<T>` deserializes a single standalone instance of `T` (one that is not encoded inside an `Array`) into an `Array` of one element (#75)
   2. Fixed `IoReader::forward_read_bytes` and `IoReader::forward_read_str` not clearing buffer after forwarding

## 0.1.4

1. Added `Deref` and `DerefMut` impl for `Array` and `Symbol`

## 0.1.3

1. Fixed [#69](https://github.com/minghuaw/fe2o3-amqp/issues/69)

## 0.1.2

1. Fixed clippy warnings except for too_many_arguments

## 0.1.1

1. Moved `Box` outside the `Described`

## 0.1.0

1. Passed `amqp-types-test` and `amqp-large-content-test`

## 0.0.11

1. Fixed a bug where deserializing a `Value` type doesn't reset `Deserializer`'s `enum_type` field.

## 0.0.10

1. Fixed a bug with deserializing small and negative values for `i32` and `i64`

## 0.0.9

1. Fixed bug in macro where a described value of Null will be treated as if it is a field that has default value and then end up returning an error.

## 0.0.8

1. Updated dependencies
   1. `ordered-float` to "3"
   2. `uuid` to "1"

## 0.0.7

1. Added `FromIterator` impl for `Array`

## 0.0.6

1. (forgot)

## 0.0.5

1. Made `serde_amqp::read::{IoReader, SliceReader}` public.

## 0.0.4

1. Added `#![deny(missing_docs, missing_debug_implementations)]`

## 0.0.3

1. Fixed a bug where `uuid` cannot be properly deserialized because it depends on `is_human_readable()` to figure out whether the encoded format is binary or string.

## 0.0.2

1. Fixed documentation bug

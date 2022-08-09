# Change Log

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

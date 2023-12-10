# Changelog

## 0.9.0

1. Unified versioning with other `fe2o3-amqp` crates

## 0.6.0

1. Updated `fe2o3-amqp-types` to version "0.7.0" which introduced breaking changes to the type alias
   `FilterSet` to support legacy formatted filter set.

## 0.5.0

1. Updated deps to breaking alpha releases

## 0.4.0

1. Breaking change(s):
   1. Updated `serde_amqp` to "0.4.0" which introduced breaking change that `Value::Map` now wraps around a `OrderedMap` #96
   2. Changed the following type to either alias or use an `OrderedMap` instead of `BTreeMap` to preserve the encoded order after deserialization #96
      1. `LegacyAmqpHeadersBinding`
2. Re-exporting `OrderedMap`

## 0.3.1

1. Updated `serde_amqp` to "0.3.1"
2. Updated `fe2o3-amqp-types` to "0.4.1" which fixed [#95](https://github.com/minghuaw/fe2o3-amqp/issues/95)

## 0.3.0

1. Updated `fe2o3-amqp-types` version to "0.4.0"

## 0.2.1

1. Added `descriptor_code()` and `descriptor_name` methods

## 0.2.0

1. Updated `serde_amqp` to 0.2.1 which introduced breaking bug fixes
   1. `Array<T>` deserializes a single standalone instance of `T` (one that is not encoded inside an `Array`) into an `Array` of one element (#75)
   2. Fixed `IoReader::forward_read_bytes` and `IoReader::forward_read_str` not clearing buffer after forwarding

## 0.1.3

1. Updated deps version
   1. `fe2o3-amqp-types` to "^0.2.1"
   2. `serde_amqp` to "^0.1.4"

## 0.1.2

1. Added `impl From<filter_type> for Described<Value>` and `impl From<filter_type> for Option<Described<Value>>`

## 0.1.1

1. Added `impl From<_> for Described<Value>` for all filter types

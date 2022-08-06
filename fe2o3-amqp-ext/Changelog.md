# Changelog

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

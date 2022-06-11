# Change Log

## 0.0.17

1. Renamed `Message::body_section` to `body`
2. Renamed `BodySection` to `Body`

## 0.0.16

1. Added `PartialEq` and `PartialOrd` to `TxnCapability`

## 0.0.15

1. Made target's fields public

## 0.0.14

1. Added `Display` impl for some message format types

## 0.0.13

1. Fixed a bug where field `global_id` of `Declare` should be and `Option`

## 0.0.9 ~ 0.0.12

1. Added types defined in transactions

## 0.0.7

1. Updated dependency `serde_amqp` to `"^0.0.5"`.

## 0.0.6

1. Added `#![deny(missing_debug_implementations)]`

## 0.0.5

1. Moved `ConnectionState` and `SessionState` from `fe2o3-amqp` to `fe2o3-amqp-types` as they are defined in the specification.

# Change Log

## 0.1.2

1. Change `serde_amqp` version requirement to "^0.1.1";

## 0.1.1

1. Relaxed `serde_amqp` version requirement to "0.1"  

## 0.1.0

1. Passed `amqp-types-test` and `amqp-large-content-test`

## 0.0.30

1. Removed `Maybe` type but kept `trait DecodeIntoMessage` for potentially allowing user to customize message decoding
   1. `Maybe` is removed because it makes AmqpSequence a vec of `Maybe<T>` when it doesn't have to
2. Added `Body::Nothing` variant

## 0.0.29

1. Moved `trait DecodeIntoMessage` from `fe2o3-amqp` into `fe2o3-amqp-types`

## 0.0.28

1. Added `Maybe` type, which is like `Option`, to allow deserializing no-body section messages. [#49](https://github.com/minghuaw/fe2o3-amqp/issues/49)

## 0.0.27

1. Updated `serde_amqp` to version 0.0.11
2. Added convenience builder functions for ([`DeliveryAnnotations`], [`MessageAnnotations`], [`Footer`], [`ApplicationProperties`])

## 0.0.26

1. Updated `serde_amqp` to version 0.0.10

## 0.0.25

1. Updated `serde_amqp` to version 0.0.9

## 0.0.24

1. Added convenience functions to handle delivery `DeliveryState`

## 0.0.23

1. Added convenience functions to handle delivery `Outcome`

## 0.0.22

1. Added convenience functions `is_value()`, `is_sequence()`, `is_data` for message `Body`

## 0.0.21

1. Added derive trait of `PartialOrd, PartialEq, Eq, Ord, Hash` for `Role`

## 0.0.20

1. Added impl of `From<Outcome> for DeliveryState`

## 0.0.19

1. Added impl of `From<TransactionError> for ErrorCondition`

## 0.0.18

1. Fixed bug where multiple valued fields should be encoded as `Array` instead of `List`
2. Manually implemented `Serialize` and `Deserialize` for `SaslMechanism` to make sure `sasl_server_mechanisms` field contains at least value

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

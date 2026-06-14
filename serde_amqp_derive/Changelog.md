# Changelog

## Unreleased

1. Added a `#[amqp_contract(multiple)]` field attribute. On a `multiple`
   (`Option<Array<T>>`) field, a zero-length array is normalized to `None` on
   deserialization so that null and an empty array decode identically, as
   required by the AMQP spec (issue
   [#111](https://github.com/minghuaw/fe2o3-amqp/issues/111)).

## 0.3.0

1. Updated deps

## 0.2.1

1. Allow using raw u64 as the descriptor code (ie. `0x0000_0000_0000_0000`)

## 0.2.0

1. Fixed [#117](https://github.com/minghuaw/fe2o3-amqp/issues/117)

## 0.1.2

1. Fixed generated `where` clause not using re-exported `serde`

## 0.1.1

1. Fixed clippy warnings except for too_many_arguments

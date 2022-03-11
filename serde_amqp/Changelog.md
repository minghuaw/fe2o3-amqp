# Change Log

## 0.0.5

1. Made `serde_amqp::read::{IoReader, SliceReader}` public.

## 0.0.4

1. Added `#![deny(missing_docs, missing_debug_implementations)]`

## 0.0.3

1. Fixed a bug where `uuid` cannot be properly deserialized because it depends on `is_human_readable()` to figure out whether the encoded format is binary or string.

## 0.0.2

1. Fixed documentation bug

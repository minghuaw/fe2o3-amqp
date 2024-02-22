# Changelog

## 0.2.3

1. Backported 0.9.1

## 0.9.1

1. Use explicit `swap_remove` instead of deprecated `remove`
2. Updated deps
   1. `fe2o3-amqp` to 0.9.3
   2. `fe2o3-amqp-types` to 0.9.1

## 0.9.0

1. Unified versioning with other `fe2o3-amqp` crates

## 0.2.2

1. Ported 0.1.2

## 0.1.2

1. Added `detach_then_resume_on_session()`

## 0.2.1

1. Ported 0.1.1

## 0.1.1

1. Fixed potential add/sub with overflow by using wrapping/checked/saturating add/sub

## 0.2.0

1. Updated `fe2o3-amqp-types` to version "0.7.0" and `fe2o3-amqp` to version "0.8.0", which
    introduced breaking change to the type alias `FilterSet` to support legacy formatted filter set.

## 0.1.0

1. Added documentation

## 0.0.5

1. Checking both `"status-code"` and `"statusCode"` because multiple cases have been found in different implementations

## 0.0.4

1. Removed `MessageSerializer` and`MessageDeserializer` traits
2. Added `Request` and `Response` traits
3. Added `call` method to `MgmtClient` and removed all operation methods

## 0.0.2

1. Changed "statusCode" to "status-code" and "statusDescription" to "status-description"
2. Added methods for each operation (`create`, `read`, etc.)

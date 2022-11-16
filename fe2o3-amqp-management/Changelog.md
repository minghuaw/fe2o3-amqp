# Changelog

## 0.0.5

1. Checking both `"status-code"` and `"statusCode"` because multiple cases have been found in different implementations

## 0.0.4

1. Removed `MessageSerializer` and`MessageDeserializer` traits
2. Added `Request` and `Response` traits
3. Added `call` method to `MgmtClient` and removed all operation methods

## 0.0.2

1. Changed "statusCode" to "status-code" and "statusDescription" to "status-description"
2. Added methods for each operation (`create`, `read`, etc.)

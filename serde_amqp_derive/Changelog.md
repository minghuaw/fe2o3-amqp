# Changelog

## 0.2.1

1. Allow the macro to work for both described types and non-described types. If no descriptor name
or descriptor code is found, it will treat the type as a non-described type. For non-described types,
the trait impls become similar to the `serde`'s `Serialize` and `Deserialize` traits but allow the
users to specify the encoding of the type ("list" or "map").

## 0.2.0

1. Fixed [#117](https://github.com/minghuaw/fe2o3-amqp/issues/117)

## 0.1.2

1. Fixed generated `where` clause not using re-exported `serde`

## 0.1.1

1. Fixed clippy warnings except for too_many_arguments

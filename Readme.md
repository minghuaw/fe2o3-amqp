# `fe2o3-amqp`

An implementation of the AMQP1.0 protocol based on serde and tokio.

## Quick start

```rust
// TODO
```

## Components

| Name | Description |
|------|-------------|
|`serde_amqp_derive`| Custom derive macro for described types as defined in AMQP1.0 protocol |
|`serde_amqp`| AMQP1.0 serializer and deserializer as well as primitive types |
|`fe2o3-amqp-types`| AMQP1.0 data types |
|`fe2o3-amqp`| Implementation of AMQP1.0 `Connection`, `Session`, and `Link` |
|`fe2o3-amqp-ext`| Extension types and implementations |

## Road map

- [ ] Proper error handling
- [ ] Pipelined open
- [ ] SASL SCRAM-SHA1
- [ ] Transaction
- [ ] Listeners
- [ ] Link re-attachment

# fe2o3-amqp-types

Implements AMQP1.0 data types as defined in the core [specification](http://docs.oasis-open.org/amqp/core/v1.0/os/amqp-core-overview-v1.0-os.html).

## Feature flags

Please note that `Performative` will require both `"transport"` and `"messaging"` feature flags
enabled.

- `"primitive"`: enables the primitive types defined in part 1.6 in the core specification.
- `"transport"`: enables most of the types defined in part 2.4, 2.5, and 2.8 of the core specifiction.
- `"messaging"`: enables the types defined in part 2.7 and part 3 defined in the core specification
- `"transaction"`: enables the types defined in part 4.5 of the core specification
- `"security"`: enables the types defined in part 5 of the core specifiction.

```toml
default = [
    "primitive",
    "transport",
    "messaging",
    "security",
]
```

License: MIT/Apache-2.0

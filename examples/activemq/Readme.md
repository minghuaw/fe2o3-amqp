# Example usage with Active MQ

This example assumes you have an ActiveMQ instant that supports AMQP 1.0
running on your localhost

`ActiveMQ` uses alternative TLS establishment (ie. establish TLS without
exchanging ['A', 'M', 'Q', 'P', '2', '1', '0', '0'] header). Because of this,
you do **NOT** need to enable either "native-tls" or "rustls" feature flag.

Please note that you may need to explicitly set you `ActiveMQ` to use TLSv1.2 or higher
in the xml configuration file.

```xml
<transportConnector name="amqp+ssl" uri="amqp+ssl://0.0.0.0:5671?transport.enabledProtocols=TLSv1.2"/>
```

## Run examples

You can run the examples with the following commands

```bash
cargo run --bin tcp
```

```bash
cargo run --bin native_tls
```

```bash
cargo run --bin rustls
```

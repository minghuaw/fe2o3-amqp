# Example usage with Active MQ

This example assumes you have an ActiveMQ instant that supports AMQP 1.0
running on your localhost

`ActiveMQ` uses alternative TLS establishment (ie. establish TLS without
exchanging ['A', 'M', 'Q', 'P', '2', '1', '0', '0'] header).

- The `"rustls"` example shows the more complicated way to perform alternative TLS establishment - manually/explicitly establish a `tls_stream` and then pass it to `Connection`. This also means that you do not need to enable the "rustls" feature flag.

- The `"native_tls"` example shows how to use a config to ask the `Connection` to do this implicitly. The user should also check the `service_bus` example to see how to establish alternative TLS connection implicitly, which also means you need to enable the "native-tls" feature flag.

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

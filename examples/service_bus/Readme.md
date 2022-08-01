# Service Bus Example

This is a simple example showcasing how to send message to and receiver message from Azure Service Bus.

To run the example, the following environment variables must be set:

- `HOST=<namespace>.servicebus.windows.net`
- `USER=<SharedAccessKeyName>`
- `PASSWORD=<SharedAccessKey>`

Replace the field wrapped in `<>` with the corresponding value for your Service Bus instance.

Then you can run the example with

```sh
cargo run --bin sender
```

OR

```sh
cargo run --bin receiver
```

# Service Bus Example

This is a simple example showcasing how to send message to and receiver message from Azure Service Bus.

To run the example, the following environment variables must be set in an `.env` file:

- `HOST=<namespace>.servicebus.windows.net`
- `SAS_KEY_NAME=<SharedAccessKeyName>`
- `SAS_KEY_VALUE=<SharedAccessKey>`
- `QUEUE_NAME=<queue>`

Replace the field wrapped in `<>` with the corresponding value for your Service Bus instance.

Then you can run the example with

```sh
cargo run --bin sender
```

OR

```sh
cargo run --bin receiver
```

# Event Hub Example

This is a simple example demonstrating how to work with Azure Event Hub with fe2p3-amqp.

## Basic Usage

To run the example(s), the following environment variables must be set in a `.env` file:

- `HOST_NAME=<namespace>.servicebus.windows.net`
- `SHARED_ACCESS_KEY_NAME=<SharedAccessKeyName>`
- `SHARED_ACCESS_KEY_VALUE=<SharedAccessKey>`
- `EVENT_HUB_NAME=<event_hub_name>`

Replace the field wrapped in `<>` with the corresponding value for your Event Hub instance.

Then you can run the example with

```sh
cargo run --bin simple_sender
```

OR

```sh
cargo run --bin simple_receiver
```

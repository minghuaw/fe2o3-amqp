# Event Hubs Example

## Alternative: 

[azeventhubs](https://crates.io/crates/azeventhubs) is built on top of `fe2o3-amqp` and should look familiar for those who have used the SDK for dotnet or golang.

## Basic Usage

To run the example(s), the following environment variables must be set in a `.env` file:

- `HOST_NAME=<namespace>.servicebus.windows.net`
- `SHARED_ACCESS_KEY_NAME=<SharedAccessKeyName>`
- `SHARED_ACCESS_KEY_VALUE=<SharedAccessKey>`
- `EVENT_HUB_NAME=<event_hub_name>`

Replace the fields wrapped in `<>` with the corresponding values for your Event Hub instance.

Then you can run the example with

```sh
cargo run --bin simple_sender
```

```sh
cargo run --bin simple_receiver
```

```sh
cargo run --bin send_to_partition
```

OR

```sh
cargo run --bin receiver_with_filter
```

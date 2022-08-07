# Examples

The examples below are individual examples which you need to `cd` into the corresponding directory to run the example with `cargo run`

## Basic Usage

| Name | Description |
|------|-------------|
|[sender](./sender/) | A simple sender with default configuration |
|[dynamic_sender](./dynamic_sender) | Request the receiving peer to dynamically create a node at target |
|[send_with_custom_properties](./send_with_custom_properties) | Send a message with customized message sections |
|[receiver](./receiver/) | A simple receiver with default configuration |
|[receiver_auto_accept](./receiver_auto_accept/) | A simple receiver that accepts incoming deliveries automatically |
|[dynamic_receiver](./dynamic_receiver) | Request the sending peer to dynamically create a node at source |
|[recv_with_filter](./recv_with_filter) | Receive message with filter |
|[batchable_send](./batchable_send/)| A simple sender that sends multiple messages but doesn't require immediate disposition |
|[dispose_multiple](./dispose_multiple) | A simple receiver that disposes multiple deliveries in one Disposition frame (if all deliveries are consecutive) |
|[listener](./listener)| A simple listener that handles incoming connections, sessions, and links |

## TLS and SASL

| Name | Description |
|------|-------------|
|[rustls_connection](./rustls_connection/)|Establish TLS connection with default and custom rustls connectors|
|[native_tls_connection](./native_tls_connection)|Establish TLS connection with default and custom native-tls connectors |
|[alternative_tls_connection](./alternative_tls_connection/)|Alternative establishment of TLS connection (core spec 5.2.1)|
|[sasl_connection](./sasl_connection/) |Establish connection with SASL Plain|
|[tls_sasl_connection](./tls_sasl_connection/) |Establish connection with default rustls connector and SASL Plain|
|[sasl_listener](./sasl_listener/) |SASL layer on listener|

## Transaction

| Name | Description |
|------|-------------|
|[txn_posting](./txn_posting/)|Transactional posting with an explicit control link|
|[owned_txn_posting](./owned_txn_posting)|Transactional posting with an implicit control link|
|[txn_retirement](./txn_retirement)|Transactional retirement with an explicit control link|
|[owned_txn_retirement](./owned_txn_retirement)|Transactional retirement with an implicit control link|
|[txn_enabled_listener](./txn_enabled_listener/)|Allow handling remotely initated transaction on the listener|

## Other Examples

| Name | Description |
|------|-------------|
|[websocket](./websocket/) | AMQP 1.0 over websocket |
|[service_bus](./service_bus/) | Sending to / receiving from Azure Service Bus |
|[service_bus_over_websocket](./service_bus_over_websocket) | Sending to / receiving from Azure Service Bus over WebSocket |
|[event_hub](./event_hub) | Send to Azure Event Hub (WIP) |

## More examples coming

- TLS layer with rustls on listner
- TLS layer with native-tls on listner
- TLS + SASL on listner

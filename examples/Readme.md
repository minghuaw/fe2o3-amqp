# Examples

The examples below are individual examples which you need to `cd` into the corresponding directory to run the example with `cargo run`

## Basic Usage

| Name | Description |
|------|-------------|
|[sender](./sender/) | A simple sender with default configuration |
|[receiver](./receiver/) | A simple receiver with default configuration |
|[batchable_send](./batchable_send/)| A simple sender that sends a message but doesn't require immediate disposition |
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

## Upcoming examples

- TLS layer with rustls on listner
- TLS layer with native-tls on listner
- TLS + SASL on listner

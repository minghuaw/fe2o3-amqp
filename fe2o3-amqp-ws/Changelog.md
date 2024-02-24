# fe2o3-amqp-ws

## 0.10.0

1. Unified versioning with other `fe2o3-amqp` crates

## 0.9.0

1. Unified versioning with other `fe2o3-amqp` crates
2. Updated `http` to "1"
3. Updated `tungstenite` and `tokio-tungstenite` to "0.21"

## 0.4.0

1. Updated `tungstenite` and `tokio-tungstenite` to "0.20.1", which fixes [CVE-2023-43669](https://github.com/snapview/tungstenite-rs/pull/379).

### Breaking Changes

1. As part of the upgrade, `connect_with_config()` and `connect_tls_with_config()` are added with one argument `disable_nagle: bool`.

## 0.3.1

1. Updated readme to include `wasm32-unknown-unknown` example

## 0.3.0

1. Initial `wasm32-unknown-unknown` support
2. `WebSocketStream::connect` now returns a single `Self` instead of a tuple for the `Ok` variant.

## 0.2.0

1. Updated `tungstenite` and `tokio-tungstenite` to "0.18"

## 0.1.2

1. Relaxed dependency versions

## 0.1.1

1. Fixed typo in docs

## 0.1.0

1. Initial release

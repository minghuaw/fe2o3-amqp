# fe2o3-amqp-ws

## Unreleased

1. Updated `tungstenite` and `tokio-tungstenite` from `0.26` to `0.30`.
2. Removed the TLS feature gate from `WebSocketStream::connect_tls_with_config`, `WebSocketStream::connect_tls_with_stream`, and `WebSocketStream::connect_tls_with_stream_and_config` (issue [#356](https://github.com/minghuaw/fe2o3-amqp/issues/356)). The three functions now exist in every feature build, so a library crate can depend on `fe2o3-amqp-ws` with no TLS feature and let the application select the TLS stack. With a TLS feature enabled, their behavior does not change. With no TLS feature enabled, a `ws://` address connects in plaintext, and a `wss://` address returns `tungstenite::error::UrlError::TlsFeatureNotEnabled` before this crate opens a socket. This crate never downgrades a `wss://` address to a plaintext connection. `WebSocketStream::connect` and `WebSocketStream::connect_with_config` keep their current behavior, because they take no connector.
3. Relaxed the bound on `impl WebSocketStream<TokioWebSocketStream<MaybeTlsStream<S>>>` and dropped `Sync`, which `tokio_tungstenite::client_async_tls_with_config` never required.

### Breaking Changes

1. **Re-exports**: Added `pub use tungstenite` and `pub use tokio_tungstenite`.
2. **`WsMessage` removed**: The `WsMessage` newtype wrapper has been removed. Stream and Sink impls now use `tungstenite::Message` directly (accessible via the re-export `fe2o3_amqp_ws::tungstenite::Message`). Users who accessed `msg.0`, called `.into_inner()`, `.as_inner()`, or relied on the `Deref`/`From` impls should now use `tungstenite::Message` directly.
3. **Stream/Sink error types**: `TokioWebSocketStream`, `WasmWebSocketStream`, and `WebSocketStream` trait impls now use `crate::Error` instead of `tungstenite::Error`.
4. **`Error` simplified**: All tungstenite-mirrored variants (`ConnectionClosed`, `Io`, `Tls`, `Capacity`, `Protocol`, `WriteBufferFull`, `Utf8`, `Url`, `Http`, `HttpFormat`, `AttackAttempt`) collapsed into a single `Tungstenite(tungstenite::Error)` variant. Match on `Error::Tungstenite(e)` and then on `e` for tungstenite-specific handling.

## 0.16.0

1. Bumped version to "0.16.0" to track `fe2o3-amqp` 0.16.0, which drops the `ring`
   dependency entirely (issue
   [#333](https://github.com/minghuaw/fe2o3-amqp/issues/333)). This crate has no
   source change.

## 0.14.0

1. Put wrapped type of `Error::Http(_)` behind `Box` to avoid `clippy::result-large-err` (PR #313)
2. Updated depdencies (PR #315)

## 0.13.0

1. Updated deps

## 0.12.0

1. Updated deps

## 0.11.0

1. Updated deps.
2. Fixed bug in doc example

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

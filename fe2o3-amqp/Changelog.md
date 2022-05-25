# Change Log

## 0.0.10

1. Reduced lookups needed for incoming session frames and link frames

## 0.0.9

1. Implemented client side transaction
2. Removed redundant `impl Into<Type>` in the arguments when the `Type` itself doesn't really have anything to convert from
3. Fixed most clippy warnings

## 0.0.8

1. Changed internal LinkState
2. Fixed a bug where link may send detach twice when dropping
3. Updated dependency `fe2o3-amqp-types` for upcoming transaction implementation

## 0.0.7

1. Added `ConnectionAcceptor`, `SessionAcceptor` and `LinkAcceptor`
2. Added, naive listener side PLAIN SASL mechanism.
3. Removed type state from `Sender` and `Receiver`. Whether link is closed by remote is check at runtime.

## 0.0.6

1. Fixed errors in documentation about TLS protocol header
2. Removed `"rustls"` from default features
3. Changed feature dependend `connection::Builder::tls_connector()` method to aliases to `rustls_connector` and `native_tls_connector`

## 0.0.5

1. Removed unused dependency `crossbeam`

## 0.0.4

1. TLS is only supported if either "rustls" or "native-tls" feature is enabled.
2. Default TlsConnector will depend on the the particular feature enabled.
3. `Connection` `Builder`'s `client_config` field is now replaced with `tls_connector` which allows user to supply custom `TlsConnector` for TLS handshake.

## 0.0.3

1. Made session and link's errors public

## 0.0.2

1. Added `#![deny(missing_docs, missing_debug_implementations)]`
2. Added documentations and short examples in the documentations

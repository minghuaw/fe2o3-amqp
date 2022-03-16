# Change Log

## 0.0.4

1. TLS is only supported if either "rustls" or "native-tls" feature is enabled.
2. Default TlsConnector will depend on the the particular feature enabled.
3. `Connection` `Builder`'s `client_config` field is now replaced with `tls_connector` which allows user to supply custom `TlsConnector` for TLS handshake.

## 0.0.3

1. Made session and link's errors public

## 0.0.2

1. Added `#![deny(missing_docs, missing_debug_implementations)]`
2. Added documentations and short examples in the documentations

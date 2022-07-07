# Change Log

## 0.0.26

1. Updated `fe2o3-amqp-types` to version 0.0.30 which removed `Maybe` but added `Body::Nothing` variant to deal with no-body section message.

## 0.0.25

1. Updated `fe2o3-amqp-types` to version "0.0.29" which added `Maybe` to allow no-body section message.

## 0.0.24

1. Removed debug printouts

## 0.0.23

1. Updated `serde_amqp` to 0.0.11
2. Updated `fe2o3-amqp-types` to 0.0.27

## 0.0.22

1. Updated `serde_amqp` to version 0.0.10
2. Updated `fe2o3-amqp-types` to version 0.0.26

## 0.0.21

1. Updated `serde_amqp` to version 0.0.9

## 0.0.20

1. API changes
   1. `Sender`'s `send()`, `send_batchable()`, and `send_with_timeout()` now returns the `Result<Outcome, SendError>` which carries the outcome of the delivery. `SendError` no longer has variants that marks failed delivery state. It is the user's responsibility to check whether the delivery is accepted or not
   2. `Transaction`'s and `OwnedTransaction`'s `post()` now returns `Result<Outcome, PostError>` which carries the outcome of the delivery. `PostError` no longer has variants that marks failed delivery state. It is the user's responsibility to check whether the delivery is accepted or not

## 0.0.19

1. Added convenience functions to get ownership of or reference to the value/sequence/data from a `Delivery`

## 0.0.18

1. Added `std::error::Error` impl for public error types
2. Removed redundant tracing that were initially added for debugging purpose.

## 0.0.17

1. Remove `"txn-id"` field from receiver link properties if error encountered while sending flow

## 0.0.16

1. Closing or detaching `Sender` and `Receiver` now takes ownership
2. Bug fixes
   1. Fixed bug where unsettled outgoing transfers in the session's map of unsettled delivery id and link handle may be overwritten by incoming transfer when `rcv_settle_mode` is set to `Second`
   2. Fixed bug where duplicated detach may be sent when a link tries to detach after the remote peer detaches first
   3. Fixed bug receiver builder not requiring `source` to be set to start attach [#44](https://github.com/minghuaw/fe2o3-amqp/issues/44)
3. `Transaction` and `OwnedTransaction`
   1. `commit()` and `rollback()` now takes ownership
   2. `Transaction` now holds a reference to a `Controller`
   3. `OwnedTransaction` is added to be a version of `Transaction` that owns a `Controller` instead
4. Added transaction coordinator and establishment of control link on the resource side. The resource side now supports the following transactional work
   1. Transactional posting
   2. Transactional retirement
5. Transactional acquisition on the resource side is set to not implemented because the exact behavior upon committing or rolling back of a transactional acquisition is not clear [#43](https://github.com/minghuaw/fe2o3-amqp/issues/43)

## 0.0.15

1. Added type conversions to reflect bug fix in `fe2o3-amqp-types`
   1. [#35](https://github.com/minghuaw/fe2o3-amqp/issues/35)

2. AMQP protocol header and SASL protocol header are now handled by `ProtocolHeaderCodec` which allows changing between different `Framed` codecs without losing buffered data. [#33](https://github.com/minghuaw/fe2o3-amqp/issues/33)

## 0.0.14

1. Controller verifies txn capability on attach
2. Updated `fe2o3-amqp-types` to 0.0.17 which renamed `BodySection` to `Body`

## 0.0.13

1. Updated `fe2o3-amqp-types` to 0.0.16

## 0.0.12

1. Fixed missing generics bug for using default connector for either TLS feature
2. Added checking tasks for individual features in cargo make

## 0.0.11

1. Fixed typo in Readme

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

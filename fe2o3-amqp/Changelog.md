# Change Log

## 0.4.0

1. ***Breaking*** changes:
   1. Restructured `connection::error::{OpenError, Error}` and `session::error:{BeginError, Error}`
   2. `DecodeError` variant now carries a `String` message of the `serde_amqp::Error`
2. `Connection` and non-txn `Session` no longer hold a copy of the controller sender to its own engine
3. Resuming receiver now doesn't assume the continued payload transfer starts at the receiver's `Received` state
4. Made `Sendable`'s fields public
5. Updated documentations on
   1. `Sender::send`
   2. `Sender::attach`
   3. `Receiver::attach`

## 0.3.2

1. Made traits `FromPreSettled`, `FromDeliveryState`, `FromOneshotRecvError` public for compatibility with rust versions <=1.58.0

## 0.3.1

1. Fixed bug where `ControlLinkAcceptor` defaulted to `SupportedReceiverSettleMode::Second`. It now defaults to `SupportedReceiverSettleMode::Both`
2. Added `ControlLinkAcceptor::builder()` to allow customize acceptor configs
3. Updated `serde_amqp` dep to "0.2.3" which allows using `&str` as look up keys for maps with `Symbol` as the key type

## 0.3.0

1. Updated `serde_amqp` to 0.2.1 which introduced breaking bug fixes
   1. `Array<T>` deserializes a single standalone instance of `T` (one that is not encoded inside an `Array`) into an `Array` of one element (#75)
   2. Fixed `IoReader::forward_read_bytes` and `IoReader::forward_read_str` not clearing buffer after forwarding
2. Breaking changes
   1. Receiver no longers checks whether local and remote SenderSettleMode are the same. It now simply takes the value given by the sending link.
      1. Removed `ReceiverAttachError::SndSettleModeNotSupported`
   2. Set the link credit to zero before `Receiver` sends out a closing detach.
   3. Removed `DetachError::NonDetachFrameReceived` error. Non-detach frame received while detaching will simply be logged with `tracing::debug` and then dropped
3. Added Service Bus example

## 0.2.7

1. Fixed a bug where sender would attempt to resume delivery when remote attach carries a non-null but empty unsettled map and thus resulting in `IllegalState` error
2. The connection engine will call `SinkExt::close` on the transport when stopping

## 0.2.6

1. Only override connection builder's hostname and domain if valid values are found in url.

## 0.2.5

1. Added `accept_all`, `reject_all`, `modify_all`, and `release_all` to handle disposition of multiple deliveries in a single Disposition if all deliveries are consecutive.
if the deliveries are not all consecutive, then the number of Disposition frames will be equal to the number of consecutive chunks.

## 0.2.4

1. Removed `BufReader` and `BufWriter` as `Framed` is already internally buffered. (ie. this is essentially the same as v0.2.2)

## 0.2.3

1. Wrap `Io` inside `BufReader` and `BufWriter`

## 0.2.2

1. Added `on_dynamic_source` and `on_dynamic_target` for link acceptor to handle incoming dynamic source and dynamic target.
2. Updated `serde_amqp` version requirement to "^0.1.4"
3. Updated `fe2o3-amqp-types` version requirement to "^0.2.1"

## 0.2.1

1. Make `auto_accept` field in receiver builder public

## 0.2.0

1. ***Breaking*** change
   - Added `auto_accept` option for `Receiver` and receiver `Builder` that controls whether the receiver will automatically accept incoming message
2. Drafted link resumption on client side
3. Sender and receiver on the client side checks if declared `snd_settle_mode` and `rcv_settle_mode` are supported by the remote peer
4. Fixsed most clippy warnings

## 0.1.4

1. Fixed most clippy warnings

## 0.1.3

1. #66. Avoided copying partial payload into the buffer for multi-frame delivery. In a non-scientific test, this improved performance for large content from around 1.2s to 0.8s.

## 0.1.2

1. Changed `serde_amqp` version requirement to "^0.1.1"
2. Changed `fe2o3-amqp-types` version requirement to "^0.1.2"
3. Relaxed optional dep `uuid` version requirement to "1.1"
4. Relaxed optional dep `native-tls` version requirement to "0.2"

## 0.1.1

1. Added `SaslAnonymousMechanism` to SASL acceptor

## 0.1.0

1. Passed `amqp-types-test` and `amqp-large-content-test`

## 0.0.32

1. Relaxing `tokio-util` version requirement to `"<=0.7.3"

## 0.0.31

1. Somehow fixed #56 (which is sending multiple 10 MB messages in a row fails on qdrouterd) by
   - setting the `delivery_id`, `delivery_tag`, `message_format`, `settled`, and `rcv_settle_mode` fields of the intermediate transfer performative to `None` for a multiple transfer delivery at the transport encoder.

## 0.0.30

1. Fixed large content transfer problem by
   1. Separating the length delimited codec into encoder and decoder to deal with potentially a bug upstream (tokio-rs/tokio#4815)
   2. The length delimited encoder's max frame length is set to the remote max frame size
   3. The length delimited decoder's max frame length is set to the local max frame size

## 0.0.29

1. Updated doc to make it clear that the exposed `max_frame_size` API includes the 8 bytes taken by the frame header

## 0.0.28

1. Fixed a bug where empty frame (heartbeat frame)'s header is not encoded by the new encoder introduced in v0.0.27.

## 0.0.27

1. Fixed a bug where link doesnt have max-message-size set and a large message exceeds the connection's max-frame-size. The frame encoder now automatically splits outgoing transfer + payload frame into multiple transfers

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

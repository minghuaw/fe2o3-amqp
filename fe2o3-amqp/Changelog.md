# Change Log

## 0.8.15

1. Added `spawn_local` for connection builder and session builder for `target_arch = "wasm32"`.

## 0.8.14

1. Replaced `wasm-timer` with `fluvio-wasm-timer` to work around a [`parkinglot` bug](https://github.com/tomaka/wasm-timer/pull/13).

## 0.8.13

1. Fixed docsrs build error.

## 0.8.12

1. Updated readme to include `wasm32-unknown-unknown` example

## 0.8.11

1. Experimental support for `wasm32-unknown-unknown` target (see `examples/wasm32-in-browser`).
   `Close()`/`End()` are not supported yet and the user should simply drop the
   `ConnectionHandle`/`SessionHandle` to close the connection/session.

## 0.8.10

1. Ported 0.7.24

## 0.7.24

1. Relaxed `post` and `post_batachable` to require `&self` only.

## 0.8.9

1. Ported 0.7.23

## 0.7.23

1. Updated `base64` to "0.21"

## 0.8.8

1. Ported 0.7.22

## 0.7.22

1. Use `saturating_add/sub` when computing link credit `on_incoming_flow` for sender

## 0.8.7

1. Fixed clippy warnings

## 0.8.6

1. Ported 0.7.21

## 0.7.21

1. `DeliveryFut` returned by `Sender::send_batchable` and `Sender::send_batchable_ref` now will be
   able to receive the outcome after sender is re-attached.

## 0.8.5

1. Ported 0.7.20

## 0.7.20

1. Instead of panic when `ConnectionHandle::close()`/`SessionHandle::end()` is called multiple times,
   it will now return an `Error::IllegalState`.

## 0.8.4

1. Ported 0.7.19

## 0.7.19

1. Fixed #170 and #172 by reviewing potential addition/subtraction with overflow/underflow and
   replaced with wrapping/saturating add/sub

## 0.8.3

1. Ported 0.7.18

## 0.7.18

1. Merged PR [#169](https://github.com/minghuaw/fe2o3-amqp/pull/169) which fixed panics due to
   overflowing next-outgoing-id by using wrapping add
2. Fixed a bug where sender and receiver's outgoing channel is not updated when attaching to new session

## 0.8.2

1. Ported 0.7.17

## 0.7.17

1. Fixed not implemented bug in `Receiver::detach_then_resume_on_session()`

## ~~0.8.1 (yanked)~~

1. Updated `base64` to "0.20"

## ~~0.8.0 (yanked)~~

1. Updated `fe2o3-amqp-types` to 0.7.0, which introduced breaking change to the type alias
   `FilterSet` to support legacy formatted filter set. The user would need to ensure that the
   correct format is used when constructing the filter set (more details can be found at
   [Azure/azure-amqp#231](https://github.com/Azure/azure-amqp/issues/231),
   [Azure/azure-amqp#232](https://github.com/Azure/azure-amqp/issues/232)).
2. `Sender::detach()`/`Receiver::detach()` return a `DetachedSender`/`DetachedReceiver` even if it
   encounters an error
3. Impl `std::error::Error` for `SenderResumeError` and `ReceiverResumeError`

## ~~0.7.16 (yanked)~~

1. Added `detach_then_resume_on_session` method for `Sender` and `Receiver`

## 0.7.15

1. Added `.properties()` and `.properties_mut()` methods to `Receiver` and `Sender`

## 0.7.14

1. Added `.name()` method to `Receiver` and `Sender`

## 0.7.13

1. Added `Delivery::into_parts()`

## 0.7.12

1. Added `credit_mode()` getter to `Receiver`
2. Removed unused generic type parameter from `Receiver::modify` and `Receiver::release`

## 0.7.11

1. Bumped dependencies `serde_amqp` to "0.5.2" and `fe2o3-amqp-types` to "0.6.2" which fixes [#142](https://github.com/minghuaw/fe2o3-amqp/issues/142), where `&[u8]` or `String` of length 255 may fail to serialize with error too long.

## 0.7.10

1. Derive `Clone` on `DeliveryInfo`

## 0.7.9

1. Added `message_format` and getter method `message_format()` to `Delivery` struct.

## 0.7.8

1. Fixed [#136](https://github.com/minghuaw/fe2o3-amqp/issues/136) where `send_batchable` does not
   actually set the `batchable` flag.

## 0.7.7

1. Added `Sender::send_ref()` and `Sender::send_batchable_ref()` methods to allow sending without
   taking the ownership of the message.

## 0.7.6

1. Derive `Clone` on `Sendable<T>`

## 0.7.5

1. Exposed `max_message_size()` method on both `Sender` and `Receiver`.

## 0.7.4

1. Added `Builder::credit_mode()` method to `Receiver` builder.

## 0.7.3

1. Reduced duplicated code for setting default port when not specified in the connection url
2. Updated docs of `Sender::send` and `Receiver::recv` on cancel safety

## 0.7.2

1. Set default port (5672 for "amqp" and 5671 for "amqps") if not specified in the URL.

## 0.7.1

1. Fixed [#122](https://github.com/minghuaw/fe2o3-amqp/issues/122) by not checking if target is null
   if remote peer is sender and not checking source if remote peer is receiver.

## 0.7.0

1. Upgraded `fe2o3-amqp-types` to "0.6.0", which introduced breaking changes to `Body`
   ([#115](https://github.com/minghuaw/fe2o3-amqp/issues/115), please see `fe2o3-amqp-types`
   [changelog](https://github.com/minghuaw/fe2o3-amqp/blob/main/fe2o3-amqp-types/Changelog.md) for
   more details)
2. Logging with either `tracing` or `log` is now toggled by enabling the corresponding feature

## 0.6.10

1. Updated dep version to fix a bug with `AmqpValue` on custom types
   1. `serde_amqp` to "0.4.5"
   2. `fe2o3-amqp-types` to "0.5.4"

## 0.6.9

1. Changed visibiltiy of  `sesssion::Error` and `session::BeginError` to `pub`
2. Changed visibility of internal types `LinkFlowState` and `SenderTryConsumeError` to `pub(crate)`

## 0.6.8

1. Impl `From<Infallible> for OpenError`
2. Relaxed associated type bound for url on connection and connection builder fn `open()`
   1. `open()` fn previously doesn't work if the `url` argument is actually a `url::Url` instance
      because the `TryInto::Error` type is `Infallible`. This will work in this patch.

## 0.6.7

1. Disposition of message (`accept()`, `reject()`, `release()`, `modify()`, `accept_all()`,
   `reject_all()`, `release_all()`, `modify_all()`) on the receiver side no longer requires `&mut
   self`. They only need `&self` now.
2. Allow receiver disposition methods (`accept()`, `reject()`, `release()`, `modify()`,
   `accept_all()`, `reject_all()`, `release_all()`, `modify_all()`) to take any type that implement
   `Into<DeliveryInfo>`. This includes both `Delivery<T>` and `&Delivery<T>`
3. Switched internal `RwLock` from `tokio::sync::RwLock` to `parking_lot::RwLock` to resolve cancel
   safety issue with `Receiver::recv()` [#22](https://github.com/minghuaw/fe2o3-amqp/issues/22)
   1. Switching from an async `RwLock` to a blocking `RwLock` is fine because the lock guard will
      never be held across `.await` points.

## 0.6.6

1. `on_incoming_attach` should always be evaluated before sending back `Attach`. The only lazy
   evaluation left is when parsing SaslPlainProfile from url

## ~~0.6.5~~

1. Fixed logic error with lazy evaluation of `link.on_incoming_attach` introduced in 0.6.4

## ~~0.6.4~~

1. Lazily evaluate some values and return early if a preceding value is already erroneous.

## 0.6.3

1. Updated `fe2o3-amqp-types` to "0.5.2" and `serde_amqp` to "0.4.3" which fix #103
2. Fixed some clippy warnings

## 0.6.2

1. Added `alt_tls_establishment` option to connection builder, which then asks the connection to
   implicitly perform alternative tls establishment.

## 0.6.1

1. Updated `fe2o3-amqp-types` to "0.5.1" which fixes #94.

## 0.6.0

1. Breaking change(s):
   1. Updated `serde_amqp` to "0.4.0" which introduced breaking change that `Value::Map` now wraps
      around a `OrderedMap` #96
2. Switched link's unsettled map to `OrderedMap`

## 0.5.2

1. Updated `serde_amqp` to "0.3.1"
2. Updated `fe2o3-amqp-types` to "0.4.1" which fixed
   [#95](https://github.com/minghuaw/fe2o3-amqp/issues/95)

~~## 0.5.1 (Yanked)~~

1. Added trait bound `Serialize` for `U` in `impl<T, U> From<T> for Sendable<U>`.

~~## 0.5.0 (Yanked)~~

1. Updated `fe2o3-amqp-types` to `"0.4.0"` which introduced the following breaking changes
   1. `AmqpValue`, `AmqpSequence`, `Data` no longer implement `Serialize` or `Deserialize` without
      wrapper. This change allows `Sender::send()` to take `AmqpValue`, `AmqpSequence`, or `Data` as
      a way to specify the body section type.
   2. Renamed `Body::Nothing` to `Body::Empty`. This also changed the `BodyError::IsNothing` to
      `BodyError::IsEmpty`
2. Relaxed `tokio_util` version requirement to `"0.7"` as the upstream bug causing #55 won't be
   fixed until the next breaking release.

## 0.4.3

1. Extend conversions into `Sendable<_>` to all types that implements conversion into `Message` (ie.
   `impl<T: Into<Message<U>>,U> From<T> for Sendable<U>`)

## 0.4.2

1. Implemented SCRAM authenticator on the server side

## 0.4.1

1. Implemented `SaslScramSha1`, `SaslScramSha256`, `SaslScramSha512` for the client side

## 0.4.0

1. ***Breaking*** changes:
   1. Restructured `connection::error::{OpenError, Error}` and `session::error:{BeginError, Error}`
   2. `DecodeError` variant now carries a `String` message of the `serde_amqp::Error`
2. `Connection` and non-txn `Session` no longer hold a copy of the controller sender to its own
   engine
3. Resuming receiver now doesn't assume the continued payload transfer starts at the receiver's
   `Received` state
4. Made `Sendable`'s fields public
5. Renamed `role::Sender/Receiver` to `role::SenderMarker/ReceiverMarker` to avoid confusion
6. Updated documentations on
   1. `Sender::send`
   2. `Sender::attach`
   3. `Receiver::attach`

## 0.3.2

1. Made traits `FromPreSettled`, `FromDeliveryState`, `FromOneshotRecvError` public for
   compatibility with rust versions <=1.58.0

## 0.3.1

1. Fixed bug where `ControlLinkAcceptor` defaulted to `SupportedReceiverSettleMode::Second`. It now
   defaults to `SupportedReceiverSettleMode::Both`
2. Added `ControlLinkAcceptor::builder()` to allow customize acceptor configs
3. Updated `serde_amqp` dep to "0.2.3" which allows using `&str` as look up keys for maps with
   `Symbol` as the key type

## 0.3.0

1. Updated `serde_amqp` to 0.2.1 which introduced breaking bug fixes
   1. `Array<T>` deserializes a single standalone instance of `T` (one that is not encoded inside an
      `Array`) into an `Array` of one element (#75)
   2. Fixed `IoReader::forward_read_bytes` and `IoReader::forward_read_str` not clearing buffer
      after forwarding
2. Breaking changes
   1. Receiver no longers checks whether local and remote SenderSettleMode are the same. It now
      simply takes the value given by the sending link.
      1. Removed `ReceiverAttachError::SndSettleModeNotSupported`
   2. Set the link credit to zero before `Receiver` sends out a closing detach.
   3. Removed `DetachError::NonDetachFrameReceived` error. Non-detach frame received while detaching
      will simply be logged with `tracing::debug` and then dropped
3. Added Service Bus example

## 0.2.7

1. Fixed a bug where sender would attempt to resume delivery when remote attach carries a non-null
   but empty unsettled map and thus resulting in `IllegalState` error
2. The connection engine will call `SinkExt::close` on the transport when stopping

## 0.2.6

1. Only override connection builder's hostname and domain if valid values are found in url.

## 0.2.5

1. Added `accept_all`, `reject_all`, `modify_all`, and `release_all` to handle disposition of
multiple deliveries in a single Disposition if all deliveries are consecutive. if the deliveries are
not all consecutive, then the number of Disposition frames will be equal to the number of
consecutive chunks.

## 0.2.4

1. Removed `BufReader` and `BufWriter` as `Framed` is already internally buffered. (ie. this is
   essentially the same as v0.2.2)

## 0.2.3

1. Wrap `Io` inside `BufReader` and `BufWriter`

## 0.2.2

1. Added `on_dynamic_source` and `on_dynamic_target` for link acceptor to handle incoming dynamic
   source and dynamic target.
2. Updated `serde_amqp` version requirement to "^0.1.4"
3. Updated `fe2o3-amqp-types` version requirement to "^0.2.1"

## 0.2.1

1. Make `auto_accept` field in receiver builder public

## 0.2.0

1. ***Breaking*** change
   - Added `auto_accept` option for `Receiver` and receiver `Builder` that controls whether the
     receiver will automatically accept incoming message
2. Drafted link resumption on client side
3. Sender and receiver on the client side checks if declared `snd_settle_mode` and `rcv_settle_mode`
   are supported by the remote peer
4. Fixsed most clippy warnings

## 0.1.4

1. Fixed most clippy warnings

## 0.1.3

1. #66. Avoided copying partial payload into the buffer for multi-frame delivery. In a
   non-scientific test, this improved performance for large content from around 1.2s to 0.8s.

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
   - setting the `delivery_id`, `delivery_tag`, `message_format`, `settled`, and `rcv_settle_mode`
     fields of the intermediate transfer performative to `None` for a multiple transfer delivery at
     the transport encoder.

## 0.0.30

1. Fixed large content transfer problem by
   1. Separating the length delimited codec into encoder and decoder to deal with potentially a bug
      upstream (tokio-rs/tokio#4815)
   2. The length delimited encoder's max frame length is set to the remote max frame size
   3. The length delimited decoder's max frame length is set to the local max frame size

## 0.0.29

1. Updated doc to make it clear that the exposed `max_frame_size` API includes the 8 bytes taken by
   the frame header

## 0.0.28

1. Fixed a bug where empty frame (heartbeat frame)'s header is not encoded by the new encoder
   introduced in v0.0.27.

## 0.0.27

1. Fixed a bug where link doesnt have max-message-size set and a large message exceeds the
   connection's max-frame-size. The frame encoder now automatically splits outgoing transfer +
   payload frame into multiple transfers

## 0.0.26

1. Updated `fe2o3-amqp-types` to version 0.0.30 which removed `Maybe` but added `Body::Nothing`
   variant to deal with no-body section message.

## 0.0.25

1. Updated `fe2o3-amqp-types` to version "0.0.29" which added `Maybe` to allow no-body section
   message.

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
   1. `Sender`'s `send()`, `send_batchable()`, and `send_with_timeout()` now returns the
      `Result<Outcome, SendError>` which carries the outcome of the delivery. `SendError` no longer
      has variants that marks failed delivery state. It is the user's responsibility to check
      whether the delivery is accepted or not
   2. `Transaction`'s and `OwnedTransaction`'s `post()` now returns `Result<Outcome, PostError>`
      which carries the outcome of the delivery. `PostError` no longer has variants that marks
      failed delivery state. It is the user's responsibility to check whether the delivery is
      accepted or not

## 0.0.19

1. Added convenience functions to get ownership of or reference to the value/sequence/data from a
   `Delivery`

## 0.0.18

1. Added `std::error::Error` impl for public error types
2. Removed redundant tracing that were initially added for debugging purpose.

## 0.0.17

1. Remove `"txn-id"` field from receiver link properties if error encountered while sending flow

## 0.0.16

1. Closing or detaching `Sender` and `Receiver` now takes ownership
2. Bug fixes
   1. Fixed bug where unsettled outgoing transfers in the session's map of unsettled delivery id and
      link handle may be overwritten by incoming transfer when `rcv_settle_mode` is set to `Second`
   2. Fixed bug where duplicated detach may be sent when a link tries to detach after the remote
      peer detaches first
   3. Fixed bug receiver builder not requiring `source` to be set to start attach
      [#44](https://github.com/minghuaw/fe2o3-amqp/issues/44)
3. `Transaction` and `OwnedTransaction`
   1. `commit()` and `rollback()` now takes ownership
   2. `Transaction` now holds a reference to a `Controller`
   3. `OwnedTransaction` is added to be a version of `Transaction` that owns a `Controller` instead
4. Added transaction coordinator and establishment of control link on the resource side. The
   resource side now supports the following transactional work
   1. Transactional posting
   2. Transactional retirement
5. Transactional acquisition on the resource side is set to not implemented because the exact
   behavior upon committing or rolling back of a transactional acquisition is not clear
   [#43](https://github.com/minghuaw/fe2o3-amqp/issues/43)

## 0.0.15

1. Added type conversions to reflect bug fix in `fe2o3-amqp-types`
   1. [#35](https://github.com/minghuaw/fe2o3-amqp/issues/35)

2. AMQP protocol header and SASL protocol header are now handled by `ProtocolHeaderCodec` which
   allows changing between different `Framed` codecs without losing buffered data.
   [#33](https://github.com/minghuaw/fe2o3-amqp/issues/33)

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
2. Removed redundant `impl Into<Type>` in the arguments when the `Type` itself doesn't really have
   anything to convert from
3. Fixed most clippy warnings

## 0.0.8

1. Changed internal LinkState
2. Fixed a bug where link may send detach twice when dropping
3. Updated dependency `fe2o3-amqp-types` for upcoming transaction implementation

## 0.0.7

1. Added `ConnectionAcceptor`, `SessionAcceptor` and `LinkAcceptor`
2. Added, naive listener side PLAIN SASL mechanism.
3. Removed type state from `Sender` and `Receiver`. Whether link is closed by remote is check at
   runtime.

## 0.0.6

1. Fixed errors in documentation about TLS protocol header
2. Removed `"rustls"` from default features
3. Changed feature dependend `connection::Builder::tls_connector()` method to aliases to
   `rustls_connector` and `native_tls_connector`

## 0.0.5

1. Removed unused dependency `crossbeam`

## 0.0.4

1. TLS is only supported if either "rustls" or "native-tls" feature is enabled.
2. Default TlsConnector will depend on the the particular feature enabled.
3. `Connection` `Builder`'s `client_config` field is now replaced with `tls_connector` which allows
   user to supply custom `TlsConnector` for TLS handshake.

## 0.0.3

1. Made session and link's errors public

## 0.0.2

1. Added `#![deny(missing_docs, missing_debug_implementations)]`
2. Added documentations and short examples in the documentations

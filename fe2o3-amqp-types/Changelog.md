# Change Log

## 0.9.0

1. Unified versioning with other `fe2o3-amqp` crates
2. Updated `ordered-float` to "4"

## 0.7.1

1. Derive `Hash` for `Message` and related types

## 0.7.0

1. Changed type alias `FilterSet` to

   ```rust
   pub type FilterSet = OrderedMap<Symbol, Value>;
   ```

   to support legacy formatted filter set. The user would need to ensure that the correct format is
   used when constructing the filter set (more details can be found at
   [Azure/azure-amqp#231](https://github.com/Azure/azure-amqp/issues/231),
   [Azure/azure-amqp#232](https://github.com/Azure/azure-amqp/issues/232)).

## 0.6.2

1. Added impl of `BodySection` and `SerializableBody` for the following pointer types
   1. `&mut T` where `T` implements `BodySection`
   2. `Arc<T>` where `T` implements `BodySection`
   3. `Rc<T>` where `T` implements `BodySection`
   4. `Box<T>` where `T` implements `BodySection`

## 0.6.1

1. Added impl of `BodySection` and `SerializableBody` to references `&T` where `T` implements
   `BodySection`

## 0.6.0

1. Added type alias `Batch<T> = TransparentVec<T>`
2. Changed `Message::body` to generic and only implement serialization and deserialization with
   trait bound (`SerializableBody` and `DeserializableBody`)
   - `SerializableBody`/`DeserializableBody` are marker traits that are only implemented on a few
     type and are sealed from external implementation with crate only marker trait.
   - The following
     types implement `SerializableBody`:
      - `AmqpValue`
      - `AmqpSequence` / `Batch<AmqpSequence>`
      - `Data` / `Batch<Data>`
      - `Body`
   - The following types implement `DeserializableBody`
      - `AmqpValue`
      - `AmqpSequence` / `Batch<AmqpSequence>`
      - `Data` / `Batch<Data>`
      - `Body`
      - `Option<B> where B: DeserializableBody`
3. Added `IntoBody`, `FromBody`, `FromEmptyBody` that allow external types to convert to a
   serializable/deserializable `Message::body` without explicitly wrapping inside a
   `SerializableBody` or `DeserializableBody`
   - This is implemented for all types that convert into `serde_amqp::Value`
4. Allow more than one `Data` or `Sequence` section in `Body` by changing wrapped type to
   `TransparentVec`
5. Upgraded `serde_amqp` to `"0.2.0"` and changed all `#[amqp_contract(code = ... )]` to comply with
   upstream bug fix ([#117](https://github.com/minghuaw/fe2o3-amqp/issues/117))

## 0.5.4

1. Updated dep `serde_amqp` version to "0.4.5" which fixes a bug with `AmqpValue`

## 0.5.3

1. Quality of life update
   1. Added `Header::builder()`
   2. Implement `From<Builder>` for message sections and for `Option<message section>`.

## 0.5.2

1. Updated `serde_amqp` to "0.4.3", which is needed to fix #103
2. Fixed #103, and EoF will simply return a None and other errors associated with deserialization
   will be passed along.

## 0.5.1

1. Fixed #94 by switching `Annotations` map key type to `OwnedKey` to allow both `Symbol` and `u64`
   as the key type. It then uses a trait object `dyn AnnotationKey` to allow looking up the map
   using multiple types including `&str`, `String`, `u64`, `Symbol`, `SymbolRef`

## 0.5.0

1. Breaking change(s):
   1. Updated `serde_amqp` to "0.4.0" which introduced breaking change that `Value::Map` now wraps
      around a `OrderedMap` #96
   2. Changed the following types to either alias or use an `OrderedMap` instead of `BTreeMap` to
      preserve the encoded order after deserialization #96
      1. `Fields`
      2. `FilterSet`
      3. `ApplicationProperties`
      4. `Annotations` (thus any type that is a wrapper around `Annotations`)
      5. `Attach::unsettled`

## 0.4.1

1. Fixed [#95](https://github.com/minghuaw/fe2o3-amqp/issues/95)
2. Updated `serde_amqp` to "0.3.1"

## 0.4.0

1. Moved `Serialize` and `Deserialize` impl for `AmqpValue`, `AmqpSequence` and `Data` into wrapper
   types (`Serializable<T>` and `Deserializable<T>`), thus making `Message::from(AmqpSequence(_))`
   yielding a `Body::Sequence` and `Message::from(Data)` yielding a `Body::Data`
2. Renamed `Body::Nothing` to `Body::Empty`

## 0.3.6

1. Bug fix
   1. `Body::Nothing` is now serialized as `AmqpValue(Value::Null)`

## 0.3.5

1. Fixed typo

## 0.3.4

1. Added `impl TryFrom<SimpleValue> $variant_ty` for variants of `SimpleValue`

## 0.3.3

1. Added `Deref` and `DerefMut` impl to `DeliveryAnnotations`, `MessageAnnotations`, `ApplicationProperties`,
   `AmqpSequence`, and `Footer`

## 0.3.2

1. Added `Default` impl to `DeliveryAnnotations`, `MessageAnnotations`, `ApplicationProperties`,
   `AmqpSequence`, and `Footer`

## 0.3.1

1. Made `SourceBuilder` and `TargetBuilder` public

## 0.3.0

1. Updated `serde_amqp` to 0.2.1 which introduced breaking bug fixes
   1. `Array<T>` deserializes a single standalone instance of `T` (one that is not encoded inside an `Array`) into an `Array` of one element (#75)
   2. Fixed `IoReader::forward_read_bytes` and `IoReader::forward_read_str` not clearing buffer after forwarding

## 0.2.1

1. Added convenience fns for setting dynamic node prop in `Source` and `Target` when requesting for dynamic node at the remote peer.
2. Updated `serde_amqp` version requirement to "^0.1.4"

## 0.2.0

1. Bug fix and ***Breaking*** change
   1. `Attach`'s unsettled map should allow `Null` for the values (ie. `pub unsettled: Option<BTreeMap<DeliveryTag, Option<DeliveryState>>>`)
2. Fixed clippy warnings
3. Added impl `From<Received/Accepted/Released/Modified/Rejected> for DeliveryState/Outcome`

## 0.1.3

1. Added convenience fn `add_to_filter` to source builder

## 0.1.2

1. Change `serde_amqp` version requirement to "^0.1.1";

## 0.1.1

1. Relaxed `serde_amqp` version requirement to "0.1"  

## 0.1.0

1. Passed `amqp-types-test` and `amqp-large-content-test`

## 0.0.30

1. Removed `Maybe` type but kept `trait DecodeIntoMessage` for potentially allowing user to customize message decoding
   1. `Maybe` is removed because it makes AmqpSequence a vec of `Maybe<T>` when it doesn't have to
2. Added `Body::Nothing` variant

## 0.0.29

1. Moved `trait DecodeIntoMessage` from `fe2o3-amqp` into `fe2o3-amqp-types`

## 0.0.28

1. Added `Maybe` type, which is like `Option`, to allow deserializing no-body section messages. [#49](https://github.com/minghuaw/fe2o3-amqp/issues/49)

## 0.0.27

1. Updated `serde_amqp` to version 0.0.11
2. Added convenience builder functions for ([`DeliveryAnnotations`], [`MessageAnnotations`], [`Footer`], [`ApplicationProperties`])

## 0.0.26

1. Updated `serde_amqp` to version 0.0.10

## 0.0.25

1. Updated `serde_amqp` to version 0.0.9

## 0.0.24

1. Added convenience functions to handle delivery `DeliveryState`

## 0.0.23

1. Added convenience functions to handle delivery `Outcome`

## 0.0.22

1. Added convenience functions `is_value()`, `is_sequence()`, `is_data` for message `Body`

## 0.0.21

1. Added derive trait of `PartialOrd, PartialEq, Eq, Ord, Hash` for `Role`

## 0.0.20

1. Added impl of `From<Outcome> for DeliveryState`

## 0.0.19

1. Added impl of `From<TransactionError> for ErrorCondition`

## 0.0.18

1. Fixed bug where multiple valued fields should be encoded as `Array` instead of `List`
2. Manually implemented `Serialize` and `Deserialize` for `SaslMechanism` to make sure `sasl_server_mechanisms` field contains at least value

## 0.0.17

1. Renamed `Message::body_section` to `body`
2. Renamed `BodySection` to `Body`

## 0.0.16

1. Added `PartialEq` and `PartialOrd` to `TxnCapability`

## 0.0.15

1. Made target's fields public

## 0.0.14

1. Added `Display` impl for some message format types

## 0.0.13

1. Fixed a bug where field `global_id` of `Declare` should be and `Option`

## 0.0.9 ~ 0.0.12

1. Added types defined in transactions

## 0.0.7

1. Updated dependency `serde_amqp` to `"^0.0.5"`.

## 0.0.6

1. Added `#![deny(missing_debug_implementations)]`

## 0.0.5

1. Moved `ConnectionState` and `SessionState` from `fe2o3-amqp` to `fe2o3-amqp-types` as they are defined in the specification.

# serde_amqp_derive

Provides custom derive macros `SerializeComposite` and `DeserializeComposite` for described types as defined in the AMQP1.0 protocol.

## Usage

The macro provides three types of encodings:

1. `"list"`: The struct will be serialized as a described list. A described list is an AMQP1.0 list with its descriptor prepended to the list itself. The deserialization will take either the `"list"` or the `"map"` encoded values.
2. `"map"`: The struct will be serialized as a described map. A described map is an AMQP1.0 map with its descriptor prepended to the map. The deserialization will take either the `"list"` or the `"map"` encoded values.
3. `"basic"`: The struct must be a thin wrapper (containing only one field) over another serializable/deserializable type. The inner struct will be serialized/deserialized with the descriptor prepended to the struct.

### Details with the `"list"` encoding

Optinal fields

TODO:

Fields with default values:

TODO:

## Example

```rust

```
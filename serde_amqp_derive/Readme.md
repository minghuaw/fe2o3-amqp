# serde_amqp_derive

Provides custom derive macros `SerializeComposite` and `DeserializeComposite` for described types as defined in the AMQP1.0 protocol.

## Usage

The macro provides three types of encodings:

1. `"list"`: The struct will be serialized as a described list. A described list is an AMQP1.0 list with its descriptor prepended to the list itself. The deserialization will take either the `"list"` or the `"map"` encoded values.
2. `"map"`: The struct will be serialized as a described map. A described map is an AMQP1.0 map with its descriptor prepended to the map. The deserialization will take either the `"list"` or the `"map"` encoded values.
3. `"basic"`: The struct must be a thin wrapper (containing only one field) over another serializable/deserializable type. The inner struct will be serialized/deserialized with the descriptor prepended to the struct.

### Details with the `"list"` encoding

Optinal fields

If a field is not marked with `"mandatory"` in the specification, the field can be an `Option`. During serialization, the optional fields may be skipped completely or encoded as an AMQP1.0 `null` primitive (`0x40`). During deserialization, an AMQP1.0 `null` primitive or an empty field will be decoded as a `None`.

Fields with default values:

For fields that have default values defined in the specification, the field type must implement both the `Default` and `PartialEq` trait. During serialization, if the field is equal to the default value of the field type, the field will be either ignored completely or encoded as an AMQP1.0 `null` primitive (`0x40`). During deserialization, an AMQP1.0 `null` primitive or an empty field will be decoded as the default value of the type.

## Example

The `"list"` encoding will encode the `Attach` struct as a described list (a descriptor followed by a list of the fields).

```rust
/// <type name="attach" class="composite" source="list" provides="frame">
///     <descriptor name="amqp:attach:list" code="0x00000000:0x00000012"/>
#[derive(Debug, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:attach:list",
    code = 0x0000_0000_0000_0012,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Attach {
    /// <field name="name" type="string" mandatory="true"/>
    pub name: String,

    /// <field name="handle" type="handle" mandatory="true"/>
    pub handle: Handle,

    /// <field name="role" type="role" mandatory="true"/>
    pub role: Role,

    /// <field name="snd-settle-mode" type="sender-settle-mode" default="mixed"/>
    #[amqp_contract(default)]
    pub snd_settle_mode: SenderSettleMode,

    /// <field name="rcv-settle-mode" type="receiver-settle-mode" default="first"/>
    #[amqp_contract(default)]
    pub rcv_settle_mode: ReceiverSettleMode,

    /// <field name="source" type="*" requires="source"/>
    pub source: Option<Source>,

    /// <field name="target" type="*" requires="target"/>
    pub target: Option<Target>,

    /// <field name="unsettled" type="map"/>
    pub unsettled: Option<BTreeMap<DeliveryTag, DeliveryState>>,

    /// <field name="incomplete-unsettled" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub incomplete_unsettled: Boolean,

    /// <field name="initial-delivery-count" type="sequence-no"/>
    pub initial_delivery_count: Option<SequenceNo>,

    /// <field name="max-message-size" type="ulong"/>
    pub max_message_size: Option<ULong>,

    /// <field name="offered-capabilities" type="symbol" multiple="true"/>
    pub offered_capabilities: Option<Vec<Symbol>>,

    /// <field name="desired-capabilities" type="symbol" multiple="true"/>
    pub desired_capabilities: Option<Vec<Symbol>>,

    /// <field name="properties" type="fields"/>
    pub properties: Option<Fields>,
}
```

The basic encoding will have `ApplicationProperties` encoded as a descriptor followed by the wrapped element, which is a map.

```rust
/// 3.2.5 Application Properties
/// <type name="application-properties" class="restricted" source="map" provides="section">
///     <descriptor name="amqp:application-properties:map" code="0x00000000:0x00000074"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:application-properties:map",
    code = 0x0000_0000_0000_0074,
    encoding = "basic"
)]
pub struct ApplicationProperties(pub BTreeMap<String, SimpleValue>);
```

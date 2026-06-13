use serde::{Deserialize, Serialize};
use serde_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    primitives::{Array, Symbol, Uint, Ushort},
};

use crate::definitions::{Fields, IetfLanguageTag, Milliseconds};

/// Negotiate connection parameters.
/// <type name="open" class="composite" source="list" provides="frame">
///     <descriptor name="amqp:open:list" code="0x00000000:0x00000010"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:open:list",
    code = "0x0000_0000:0x0000_0010",
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Open {
    /// <field name="container-id" type="string" mandatory="true"/>
    pub container_id: String,

    /// <field name="hostname" type="string"/>
    pub hostname: Option<String>,

    /// <field name="max-frame-size" type="uint" default="4294967295"/>
    #[amqp_contract(default)]
    pub max_frame_size: MaxFrameSize,

    /// <field name="channel-max" type="ushort" default="65535"/>
    #[amqp_contract(default)]
    pub channel_max: ChannelMax,

    /// <field name="idle-time-out" type="milliseconds"/>
    pub idle_time_out: Option<Milliseconds>,

    /// <field name="outgoing-locales" type="ietf-language-tag" multiple="true"/>
    #[amqp_contract(multiple)]
    pub outgoing_locales: Option<Array<IetfLanguageTag>>,

    /// <field name="incoming-locales" type="ietf-language-tag" multiple="true"/>
    #[amqp_contract(multiple)]
    pub incoming_locales: Option<Array<IetfLanguageTag>>,

    /// <field name="offered-capabilities" type="symbol" multiple="true"/>
    #[amqp_contract(multiple)]
    pub offered_capabilities: Option<Array<Symbol>>,

    /// <field name="desired-capabilities" type="symbol" multiple="true"/>
    #[amqp_contract(multiple)]
    pub desired_capabilities: Option<Array<Symbol>>,

    /// <field name="properties" type="fields"/>
    pub properties: Option<Fields>,
}

/// Proposed maximum frame size
///
/// A simple wrapper over u32 with a default value set to `u32::MAX`
///
/// The largest frame size that the sending peer is able to accept on this connection. If this field
/// is not set it means that the peer does not impose any specific limit. A peer MUST NOT send
/// frames larger than its partner can handle. A peer that receives an oversized frame MUST close
/// the connection with the framing-error error-code.
/// Both peers MUST accept frames of up to 512 (MIN-MAX-FRAME-SIZE) octets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaxFrameSize(pub Uint);

impl Default for MaxFrameSize {
    fn default() -> Self {
        MaxFrameSize(u32::MAX)
    }
}

impl From<Uint> for MaxFrameSize {
    fn from(value: Uint) -> Self {
        Self(value)
    }
}

impl From<MaxFrameSize> for Uint {
    fn from(value: MaxFrameSize) -> Self {
        value.0
    }
}

impl From<MaxFrameSize> for usize {
    fn from(value: MaxFrameSize) -> Self {
        value.0 as usize
    }
}

/// the maximum channel number that can be used on the connection
///
/// The channel-max value is the highest channel number that can be used on the connection. This
/// value plus one is the maximum number of sessions that can be simultaneously active on the
/// connection. A peer MUST not use channel numbers outside the range that its partner can handle.
/// A peer that receives a channel number outside the supported range MUST close the connection
/// with the framing-error error-code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelMax(pub Ushort);

impl Default for ChannelMax {
    fn default() -> Self {
        Self(u16::MAX) // 65535
    }
}

impl From<Ushort> for ChannelMax {
    fn from(value: Ushort) -> Self {
        Self(value)
    }
}

impl From<ChannelMax> for Ushort {
    fn from(value: ChannelMax) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use serde_amqp::{
        from_slice,
        primitives::{Array, Symbol},
        to_vec,
    };

    use super::Open;

    fn open_with_offered(offered: Option<Array<Symbol>>) -> Open {
        Open {
            container_id: String::from("test"),
            hostname: None,
            max_frame_size: Default::default(),
            channel_max: Default::default(),
            idle_time_out: None,
            outgoing_locales: None,
            incoming_locales: None,
            offered_capabilities: offered,
            desired_capabilities: None,
            properties: None,
        }
    }

    /// Per AMQP 1.0 Part 1 (Types) section 1.4: for a field with multiplicity
    /// `multiple`, "a null value and a zero-length array (with a correct type for
    /// its elements) both describe an absence of a value and MUST be treated as
    /// semantically identical." A `multiple` field encoded as a zero-length array
    /// must therefore deserialize the same as one encoded as null, i.e. `None`.
    #[test]
    fn zero_length_multiple_field_decodes_as_none() {
        let open = open_with_offered(Some(Array::from(Vec::<Symbol>::new())));
        let buf = to_vec(&open).unwrap();
        let decoded: Open = from_slice(&buf).unwrap();

        // The empty array on the wire is an absence of a value, identical to null.
        assert_eq!(decoded.offered_capabilities, None);
    }

    /// The normalization must only collapse the empty case; a `multiple` field
    /// that actually carries elements has to round-trip unchanged.
    #[test]
    fn non_empty_multiple_field_round_trips() {
        let offered = Array::from(vec![Symbol::from("FOO"), Symbol::from("BAR")]);
        let open = open_with_offered(Some(offered.clone()));
        let buf = to_vec(&open).unwrap();
        let decoded: Open = from_slice(&buf).unwrap();

        assert_eq!(decoded.offered_capabilities, Some(offered));
    }
}

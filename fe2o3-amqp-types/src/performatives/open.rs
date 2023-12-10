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
    pub outgoing_locales: Option<Array<IetfLanguageTag>>,

    /// <field name="incoming-locales" type="ietf-language-tag" multiple="true"/>
    pub incoming_locales: Option<Array<IetfLanguageTag>>,

    /// <field name="offered-capabilities" type="symbol" multiple="true"/>
    pub offered_capabilities: Option<Array<Symbol>>,

    /// <field name="desired-capabilities" type="symbol" multiple="true"/>
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

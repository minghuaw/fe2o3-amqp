use serde_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    primitives::{Symbol, UInt, UShort},
};
use serde::{Deserialize, Serialize};

use crate::definitions::{Fields, IetfLanguageTag, Milliseconds};

/// Negotiate connection parameters.
/// <type name="open" class="composite" source="list" provides="frame">
///     <descriptor name="amqp:open:list" code="0x00000000:0x00000010"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
// #[serde(rename_all = "kebab-case")]
#[amqp_contract(
    name = "amqp:open:list",
    code = 0x0000_0000_0000_0010,
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
    pub outgoing_locales: Option<Vec<IetfLanguageTag>>,

    /// <field name="incoming-locales" type="ietf-language-tag" multiple="true"/>
    pub incoming_locales: Option<Vec<IetfLanguageTag>>,

    /// <field name="offered-capabilities" type="symbol" multiple="true"/>
    pub offered_capabilities: Option<Vec<Symbol>>,

    /// <field name="desired-capabilities" type="symbol" multiple="true"/>
    pub desired_capabilities: Option<Vec<Symbol>>,

    /// <field name="properties" type="fields"/>
    pub properties: Option<Fields>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MaxFrameSize(pub UInt);

impl Default for MaxFrameSize {
    fn default() -> Self {
        MaxFrameSize(u32::MAX)
    }
}

impl From<UInt> for MaxFrameSize {
    fn from(value: UInt) -> Self {
        Self(value)
    }
}

impl From<MaxFrameSize> for UInt {
    fn from(value: MaxFrameSize) -> Self {
        value.0
    }
}

impl From<MaxFrameSize> for usize {
    fn from(value: MaxFrameSize) -> Self {
        value.0 as usize
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ChannelMax(pub UShort);

impl Default for ChannelMax {
    fn default() -> Self {
        Self(u16::MAX) // 65535
    }
}

impl From<UShort> for ChannelMax {
    fn from(value: UShort) -> Self {
        Self(value)
    }
}

impl From<ChannelMax> for UShort {
    fn from(value: ChannelMax) -> Self {
        value.0
    }
}

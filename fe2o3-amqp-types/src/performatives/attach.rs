use std::collections::BTreeMap;

use serde_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    primitives::{Boolean, Symbol, ULong},
};

use crate::{
    definitions::{
        DeliveryTag, Fields, Handle, ReceiverSettleMode, Role, SenderSettleMode, SequenceNo,
    },
    messaging::{DeliveryState, Source, Target},
};

/// 2.7.3 Attach
/// Attach a link to a session.
/// <type name="attach" class="composite" source="list" provides="frame">
///     <descriptor name="amqp:attach:list" code="0x00000000:0x00000012"/>
///     <field name="name" type="string" mandatory="true"/>
///     <field name="handle" type="handle" mandatory="true"/>
///     <field name="role" type="role" mandatory="true"/>
///     <field name="snd-settle-mode" type="sender-settle-mode" default="mixed"/>
///     <field name="rcv-settle-mode" type="receiver-settle-mode" default="first"/>
///     <field name="source" type="*" requires="source"/>
///     <field name="target" type="*" requires="target"/>
///     <field name="unsettled" type="map"/>
///     <field name="incomplete-unsettled" type="boolean" default="false"/>
///     <field name="initial-delivery-count" type="sequence-no"/>
///     <field name="max-message-size" type="ulong"/>
///     <field name="offered-capabilities" type="symbol" multiple="true"/>
///     <field name="desired-capabilities" type="symbol" multiple="true"/>
///     <field name="properties" type="fields"/>
/// </type>
#[derive(Debug, Clone, DeserializeComposite, SerializeComposite)]
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
    ///
    /// The delivery settlement policy for the sender. When set at the receiver this
    /// indicates the desired value for the settlement mode at the sender. When set at
    /// the sender this indicates the actual settlement mode in use. The sender SHOULD
    /// respect the receiver’s desired settlement mode if the receiver initiates the
    /// attach exchange and the sender supports the desired mode.
    #[amqp_contract(default)]
    pub snd_settle_mode: SenderSettleMode,

    /// <field name="rcv-settle-mode" type="receiver-settle-mode" default="first"/>
    ///
    /// The delivery settlement policy for the receiver. When set at the sender this
    /// indicates the desired value for the settlement mode at the receiver. When set
    /// at the receiver this indicates the actual settlement mode in use. The receiver
    /// SHOULD respect the sender’s desired settlement mode if the sender initiates the
    /// attach exchange and the receiver supports the desired mode.
    #[amqp_contract(default)]
    pub rcv_settle_mode: ReceiverSettleMode,

    /// <field name="source" type="*" requires="source"/>
    ///
    /// If no source is specified on an outgoing link, then there is no source currently
    /// attached to the link. A link with no source will never produce outgoing messages
    pub source: Option<Source>,

    /// <field name="target" type="*" requires="target"/>
    ///
    /// If no target is specified on an incoming link, then there is no target currently
    /// attached to the link. A link with no target will never permit incoming messages.
    pub target: Option<Target>,

    /// <field name="unsettled" type="map"/>
    pub unsettled: Option<BTreeMap<DeliveryTag, DeliveryState>>,

    /// <field name="incomplete-unsettled" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub incomplete_unsettled: Boolean,

    /// <field name="initial-delivery-count" type="sequence-no"/>
    ///
    /// This MUST NOT be null if role is sender,
    /// and it is ignored if the role is receiver.
    /// See subsection 2.6.7.
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

#[cfg(test)]
mod tests {
    use serde_amqp::{from_slice, to_vec};

    // use crate::{
    //     definitions::{ReceiverSettleMode, Role, SenderSettleMode},
    //     messaging::Source,
    // };

    // use super::Attach;

    use crate::{
        definitions::{
            DeliveryTag, Fields, Handle, ReceiverSettleMode, Role, SenderSettleMode, SequenceNo,
        },
        messaging::{DeliveryState, Source, Target},
    };
    use serde_amqp::primitives::{Boolean, Symbol, ULong};
    use std::collections::BTreeMap;

    // #[amqp_contract(
    //     name = "amqp:attach:list",
    //     code = 0x0000_0000_0000_0012,
    //     encoding = "list",
    //     rename_all = "kebab-case"
    // )]
    pub struct Attach {
        /// <field name="name" type="string" mandatory="true"/>
        pub name: String,
        /// <field name="handle" type="handle" mandatory="true"/>
        pub handle: Handle,
        /// <field name="role" type="role" mandatory="true"/>
        pub role: Role,
        /// <field name="snd-settle-mode" type="sender-settle-mode" default="mixed"/>
        ///
        /// The delivery settlement policy for the sender. When set at the receiver this
        /// indicates the desired value for the settlement mode at the sender. When set at
        /// the sender this indicates the actual settlement mode in use. The sender SHOULD
        /// respect the receiver’s desired settlement mode if the receiver initiates the
        /// attach exchange and the sender supports the desired mode.
        // #[amqp_contract(default)]
        pub snd_settle_mode: SenderSettleMode,
        /// <field name="rcv-settle-mode" type="receiver-settle-mode" default="first"/>
        ///
        /// The delivery settlement policy for the receiver. When set at the sender this
        /// indicates the desired value for the settlement mode at the receiver. When set
        /// at the receiver this indicates the actual settlement mode in use. The receiver
        /// SHOULD respect the sender’s desired settlement mode if the sender initiates the
        /// attach exchange and the receiver supports the desired mode.
        // #[amqp_contract(default)]
        pub rcv_settle_mode: ReceiverSettleMode,
        /// <field name="source" type="*" requires="source"/>
        ///
        /// If no source is specified on an outgoing link, then there is no source currently
        /// attached to the link. A link with no source will never produce outgoing messages
        pub source: Option<Source>,
        /// <field name="target" type="*" requires="target"/>
        ///
        /// If no target is specified on an incoming link, then there is no target currently
        /// attached to the link. A link with no target will never permit incoming messages.
        pub target: Option<Target>,
        /// <field name="unsettled" type="map"/>
        pub unsettled: Option<BTreeMap<DeliveryTag, DeliveryState>>,
        /// <field name="incomplete-unsettled" type="boolean" default="false"/>
        // #[amqp_contract(default)]
        pub incomplete_unsettled: Boolean,
        /// <field name="initial-delivery-count" type="sequence-no"/>
        ///
        /// This MUST NOT be null if role is sender,
        /// and it is ignored if the role is receiver.
        /// See subsection 2.6.7.
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
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Attach {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                Attach {
                    name: ref __self_0_0,
                    handle: ref __self_0_1,
                    role: ref __self_0_2,
                    snd_settle_mode: ref __self_0_3,
                    rcv_settle_mode: ref __self_0_4,
                    source: ref __self_0_5,
                    target: ref __self_0_6,
                    unsettled: ref __self_0_7,
                    incomplete_unsettled: ref __self_0_8,
                    initial_delivery_count: ref __self_0_9,
                    max_message_size: ref __self_0_10,
                    offered_capabilities: ref __self_0_11,
                    desired_capabilities: ref __self_0_12,
                    properties: ref __self_0_13,
                } => {
                    let debug_trait_builder =
                        &mut ::core::fmt::Formatter::debug_struct(f, "Attach");
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "name",
                        &&(*__self_0_0),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "handle",
                        &&(*__self_0_1),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "role",
                        &&(*__self_0_2),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "snd_settle_mode",
                        &&(*__self_0_3),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "rcv_settle_mode",
                        &&(*__self_0_4),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "source",
                        &&(*__self_0_5),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "target",
                        &&(*__self_0_6),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "unsettled",
                        &&(*__self_0_7),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "incomplete_unsettled",
                        &&(*__self_0_8),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "initial_delivery_count",
                        &&(*__self_0_9),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "max_message_size",
                        &&(*__self_0_10),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "offered_capabilities",
                        &&(*__self_0_11),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "desired_capabilities",
                        &&(*__self_0_12),
                    );
                    let _ = ::core::fmt::DebugStruct::field(
                        debug_trait_builder,
                        "properties",
                        &&(*__self_0_13),
                    );
                    ::core::fmt::DebugStruct::finish(debug_trait_builder)
                }
            }
        }
    }
    const _: () = {
        #[automatically_derived]
        impl<'de> serde_amqp::serde::de::Deserialize<'de> for Attach {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde_amqp::serde::de::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                enum Field {
                    name,
                    handle,
                    role,
                    snd_settle_mode,
                    rcv_settle_mode,
                    source,
                    target,
                    unsettled,
                    incomplete_unsettled,
                    initial_delivery_count,
                    max_message_size,
                    offered_capabilities,
                    desired_capabilities,
                    properties,
                }
                struct FieldVisitor {}
                impl<'de> serde_amqp::serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("field identifier")
                    }
                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: serde_amqp::serde::de::Error,
                    {
                        match v {
                            "name" => Ok(Self::Value::name),
                            "handle" => Ok(Self::Value::handle),
                            "role" => Ok(Self::Value::role),
                            "snd-settle-mode" => Ok(Self::Value::snd_settle_mode),
                            "rcv-settle-mode" => Ok(Self::Value::rcv_settle_mode),
                            "source" => Ok(Self::Value::source),
                            "target" => Ok(Self::Value::target),
                            "unsettled" => Ok(Self::Value::unsettled),
                            "incomplete-unsettled" => Ok(Self::Value::incomplete_unsettled),
                            "initial-delivery-count" => Ok(Self::Value::initial_delivery_count),
                            "max-message-size" => Ok(Self::Value::max_message_size),
                            "offered-capabilities" => Ok(Self::Value::offered_capabilities),
                            "desired-capabilities" => Ok(Self::Value::desired_capabilities),
                            "properties" => Ok(Self::Value::properties),
                            _ => Err(serde_amqp::serde::de::Error::custom("Unknown identifier")),
                        }
                    }
                    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
                    where
                        E: serde_amqp::serde::de::Error,
                    {
                        match v {
                            b if b == "name".as_bytes() => Ok(Self::Value::name),
                            b if b == "handle".as_bytes() => Ok(Self::Value::handle),
                            b if b == "role".as_bytes() => Ok(Self::Value::role),
                            b if b == "snd-settle-mode".as_bytes() => {
                                Ok(Self::Value::snd_settle_mode)
                            }
                            b if b == "rcv-settle-mode".as_bytes() => {
                                Ok(Self::Value::rcv_settle_mode)
                            }
                            b if b == "source".as_bytes() => Ok(Self::Value::source),
                            b if b == "target".as_bytes() => Ok(Self::Value::target),
                            b if b == "unsettled".as_bytes() => Ok(Self::Value::unsettled),
                            b if b == "incomplete-unsettled".as_bytes() => {
                                Ok(Self::Value::incomplete_unsettled)
                            }
                            b if b == "initial-delivery-count".as_bytes() => {
                                Ok(Self::Value::initial_delivery_count)
                            }
                            b if b == "max-message-size".as_bytes() => {
                                Ok(Self::Value::max_message_size)
                            }
                            b if b == "offered-capabilities".as_bytes() => {
                                Ok(Self::Value::offered_capabilities)
                            }
                            b if b == "desired-capabilities".as_bytes() => {
                                Ok(Self::Value::desired_capabilities)
                            }
                            b if b == "properties".as_bytes() => Ok(Self::Value::properties),
                            _ => Err(serde_amqp::serde::de::Error::custom("Unknown identifier")),
                        }
                    }
                }
                impl<'de> serde_amqp::serde::de::Deserialize<'de> for Field {
                    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where
                        D: serde_amqp::serde::de::Deserializer<'de>,
                    {
                        deserializer.deserialize_identifier(FieldVisitor {})
                    }
                }
                struct Visitor {}
                impl<'de> serde_amqp::serde::de::Visitor<'de> for Visitor {
                    type Value = Attach;
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("struct amqp:attach:list")
                    }
                    fn visit_seq<A>(self, mut __seq: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde_amqp::serde::de::SeqAccess<'de>,
                    {
                        let __descriptor: serde_amqp::descriptor::Descriptor =
                            match __seq.next_element()? {
                                Some(val) => val,
                                None => {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Expecting descriptor",
                                    ))
                                }
                            };
                        match __descriptor {
                            serde_amqp::descriptor::Descriptor::Name(__symbol) => {
                                if __symbol.into_inner() != "amqp:attach:list" {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                            serde_amqp::descriptor::Descriptor::Code(__c) => {
                                if __c != 18u64 {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                        }
                        let name: String = match __seq.next_element()? {
                            Some(val) => val,
                            None => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Insufficient number of items",
                                ))
                            }
                        };
                        let handle: Handle = match __seq.next_element()? {
                            Some(val) => val,
                            None => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Insufficient number of items",
                                ))
                            }
                        };
                        let role: Role = match __seq.next_element()? {
                            Some(val) => val,
                            None => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Insufficient number of items",
                                ))
                            }
                        };
                        let snd_settle_mode: SenderSettleMode = match __seq.next_element()? {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        let rcv_settle_mode: ReceiverSettleMode = match __seq.next_element()? {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        let source: Option<Source> = match __seq.next_element()? {
                            Some(val) => val,
                            None => None,
                        };
                        let target: Option<Target> = match __seq.next_element()? {
                            Some(val) => val,
                            None => None,
                        };
                        let unsettled: Option<BTreeMap<DeliveryTag, DeliveryState>> =
                            match __seq.next_element()? {
                                Some(val) => val,
                                None => None,
                            };
                        let incomplete_unsettled: Boolean = match __seq.next_element()? {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        let initial_delivery_count: Option<SequenceNo> =
                            match __seq.next_element()? {
                                Some(val) => val,
                                None => None,
                            };
                        let max_message_size: Option<ULong> = match __seq.next_element()? {
                            Some(val) => val,
                            None => None,
                        };
                        let offered_capabilities: Option<Vec<Symbol>> =
                            match __seq.next_element()? {
                                Some(val) => val,
                                None => None,
                            };
                        let desired_capabilities: Option<Vec<Symbol>> =
                            match __seq.next_element()? {
                                Some(val) => val,
                                None => None,
                            };
                        let properties: Option<Fields> = match __seq.next_element()? {
                            Some(val) => val,
                            None => None,
                        };
                        Ok(Attach {
                            name,
                            handle,
                            role,
                            snd_settle_mode,
                            rcv_settle_mode,
                            source,
                            target,
                            unsettled,
                            incomplete_unsettled,
                            initial_delivery_count,
                            max_message_size,
                            offered_capabilities,
                            desired_capabilities,
                            properties,
                        })
                    }
                    fn visit_map<A>(self, mut __map: A) -> Result<Self::Value, A::Error>
                    where
                        A: serde_amqp::serde::de::MapAccess<'de>,
                    {
                        let mut name: Option<String> = None;
                        let mut handle: Option<Handle> = None;
                        let mut role: Option<Role> = None;
                        let mut snd_settle_mode: Option<SenderSettleMode> = None;
                        let mut rcv_settle_mode: Option<ReceiverSettleMode> = None;
                        let mut source: Option<Option<Source>> = None;
                        let mut target: Option<Option<Target>> = None;
                        let mut unsettled: Option<Option<BTreeMap<DeliveryTag, DeliveryState>>> =
                            None;
                        let mut incomplete_unsettled: Option<Boolean> = None;
                        let mut initial_delivery_count: Option<Option<SequenceNo>> = None;
                        let mut max_message_size: Option<Option<ULong>> = None;
                        let mut offered_capabilities: Option<Option<Vec<Symbol>>> = None;
                        let mut desired_capabilities: Option<Option<Vec<Symbol>>> = None;
                        let mut properties: Option<Option<Fields>> = None;
                        let __descriptor: serde_amqp::descriptor::Descriptor =
                            match __map.next_key()? {
                                Some(val) => val,
                                None => {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Expecting__descriptor",
                                    ))
                                }
                            };
                        match __descriptor {
                            serde_amqp::descriptor::Descriptor::Name(__symbol) => {
                                if __symbol.into_inner() != "amqp:attach:list" {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                            serde_amqp::descriptor::Descriptor::Code(__c) => {
                                if __c != 18u64 {
                                    return Err(serde_amqp::serde::de::Error::custom(
                                        "Descriptor mismatch",
                                    ));
                                }
                            }
                        }
                        while let Some(key) = __map.next_key::<Field>()? {
                            match key {
                                Field::name => {
                                    if name.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "name",
                                        ));
                                    }
                                    name = Some(__map.next_value()?);
                                }
                                Field::handle => {
                                    if handle.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "handle",
                                        ));
                                    }
                                    handle = Some(__map.next_value()?);
                                }
                                Field::role => {
                                    if role.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "role",
                                        ));
                                    }
                                    role = Some(__map.next_value()?);
                                }
                                Field::snd_settle_mode => {
                                    if snd_settle_mode.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "snd-settle-mode",
                                        ));
                                    }
                                    snd_settle_mode = Some(__map.next_value()?);
                                }
                                Field::rcv_settle_mode => {
                                    if rcv_settle_mode.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "rcv-settle-mode",
                                        ));
                                    }
                                    rcv_settle_mode = Some(__map.next_value()?);
                                }
                                Field::source => {
                                    if source.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "source",
                                        ));
                                    }
                                    source = Some(__map.next_value()?);
                                }
                                Field::target => {
                                    if target.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "target",
                                        ));
                                    }
                                    target = Some(__map.next_value()?);
                                }
                                Field::unsettled => {
                                    if unsettled.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "unsettled",
                                        ));
                                    }
                                    unsettled = Some(__map.next_value()?);
                                }
                                Field::incomplete_unsettled => {
                                    if incomplete_unsettled.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "incomplete-unsettled",
                                        ));
                                    }
                                    incomplete_unsettled = Some(__map.next_value()?);
                                }
                                Field::initial_delivery_count => {
                                    if initial_delivery_count.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "initial-delivery-count",
                                        ));
                                    }
                                    initial_delivery_count = Some(__map.next_value()?);
                                }
                                Field::max_message_size => {
                                    if max_message_size.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "max-message-size",
                                        ));
                                    }
                                    max_message_size = Some(__map.next_value()?);
                                }
                                Field::offered_capabilities => {
                                    if offered_capabilities.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "offered-capabilities",
                                        ));
                                    }
                                    offered_capabilities = Some(__map.next_value()?);
                                }
                                Field::desired_capabilities => {
                                    if desired_capabilities.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "desired-capabilities",
                                        ));
                                    }
                                    desired_capabilities = Some(__map.next_value()?);
                                }
                                Field::properties => {
                                    if properties.is_some() {
                                        return Err(serde_amqp::serde::de::Error::duplicate_field(
                                            "properties",
                                        ));
                                    }
                                    properties = Some(__map.next_value()?);
                                }
                            }
                        }
                        let name: String = match name {
                            Some(val) => val,
                            None => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Insufficient number of items",
                                ))
                            }
                        };
                        let handle: Handle = match handle {
                            Some(val) => val,
                            None => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Insufficient number of items",
                                ))
                            }
                        };
                        let role: Role = match role {
                            Some(val) => val,
                            None => {
                                return Err(serde_amqp::serde::de::Error::custom(
                                    "Insufficient number of items",
                                ))
                            }
                        };
                        let snd_settle_mode: SenderSettleMode = match snd_settle_mode {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        let rcv_settle_mode: ReceiverSettleMode = match rcv_settle_mode {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        let source: Option<Source> = match source {
                            Some(val) => val,
                            None => None,
                        };
                        let target: Option<Target> = match target {
                            Some(val) => val,
                            None => None,
                        };
                        let unsettled: Option<BTreeMap<DeliveryTag, DeliveryState>> =
                            match unsettled {
                                Some(val) => val,
                                None => None,
                            };
                        let incomplete_unsettled: Boolean = match incomplete_unsettled {
                            Some(val) => val,
                            None => Default::default(),
                        };
                        let initial_delivery_count: Option<SequenceNo> =
                            match initial_delivery_count {
                                Some(val) => val,
                                None => None,
                            };
                        let max_message_size: Option<ULong> = match max_message_size {
                            Some(val) => val,
                            None => None,
                        };
                        let offered_capabilities: Option<Vec<Symbol>> = match offered_capabilities {
                            Some(val) => val,
                            None => None,
                        };
                        let desired_capabilities: Option<Vec<Symbol>> = match desired_capabilities {
                            Some(val) => val,
                            None => None,
                        };
                        let properties: Option<Fields> = match properties {
                            Some(val) => val,
                            None => None,
                        };
                        Ok(Attach {
                            name,
                            handle,
                            role,
                            snd_settle_mode,
                            rcv_settle_mode,
                            source,
                            target,
                            unsettled,
                            incomplete_unsettled,
                            initial_delivery_count,
                            max_message_size,
                            offered_capabilities,
                            desired_capabilities,
                            properties,
                        })
                    }
                }
                const FIELDS: &'static [&'static str] = &[
                    serde_amqp::__constants::DESCRIPTOR,
                    "name",
                    "handle",
                    "role",
                    "snd-settle-mode",
                    "rcv-settle-mode",
                    "source",
                    "target",
                    "unsettled",
                    "incomplete-unsettled",
                    "initial-delivery-count",
                    "max-message-size",
                    "offered-capabilities",
                    "desired-capabilities",
                    "properties",
                ];
                deserializer.deserialize_struct(
                    serde_amqp::__constants::DESCRIBED_LIST,
                    FIELDS,
                    Visitor {},
                )
            }
        }
    };
    const _: () = {
        #[automatically_derived]
        impl serde_amqp::serde::ser::Serialize for Attach {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde_amqp::serde::ser::Serializer,
            {
                use serde_amqp::serde::ser::SerializeStruct;
                let mut nulls: Vec<&str> = Vec::new();
                let mut state = serializer
                    .serialize_struct(serde_amqp::__constants::DESCRIBED_LIST, 14usize + 1)?;
                state.serialize_field(
                    serde_amqp::__constants::DESCRIPTOR,
                    &serde_amqp::descriptor::Descriptor::Code(18u64),
                )?;
                for field_name in nulls.drain(..) {
                    state.serialize_field(field_name, &())?;
                }
                state.serialize_field("name", &self.name)?;
                for field_name in nulls.drain(..) {
                    state.serialize_field(field_name, &())?;
                }
                state.serialize_field("handle", &self.handle)?;
                for field_name in nulls.drain(..) {
                    state.serialize_field(field_name, &())?;
                }
                state.serialize_field("role", &self.role)?;
                if *&self.snd_settle_mode != <SenderSettleMode as Default>::default() {
                    for field_name in nulls.drain(..) {
                        state.serialize_field(field_name, &())?;
                    }
                    state.serialize_field("snd-settle-mode", &self.snd_settle_mode)?;
                } else {
                    nulls.push("snd-settle-mode");
                };
                if *&self.rcv_settle_mode != <ReceiverSettleMode as Default>::default() {
                    for field_name in nulls.drain(..) {
                        state.serialize_field(field_name, &())?;
                    }
                    state.serialize_field("rcv-settle-mode", &self.rcv_settle_mode)?;
                } else {
                    nulls.push("rcv-settle-mode");
                };
                if (&self.source).is_some() {
                    for field_name in nulls.drain(..) {
                        state.serialize_field(field_name, &())?;
                    }
                    state.serialize_field("source", &self.source)?;
                } else {
                    nulls.push("source");
                };
                if (&self.target).is_some() {
                    for field_name in nulls.drain(..) {
                        state.serialize_field(field_name, &())?;
                    }
                    state.serialize_field("target", &self.target)?;
                } else {
                    nulls.push("target");
                };
                if (&self.unsettled).is_some() {
                    for field_name in nulls.drain(..) {
                        state.serialize_field(field_name, &())?;
                    }
                    state.serialize_field("unsettled", &self.unsettled)?;
                } else {
                    nulls.push("unsettled");
                };
                if *&self.incomplete_unsettled != <Boolean as Default>::default() {
                    for field_name in nulls.drain(..) {
                        state.serialize_field(field_name, &())?;
                    }
                    state.serialize_field("incomplete-unsettled", &self.incomplete_unsettled)?;
                } else {
                    nulls.push("incomplete-unsettled");
                };
                if (&self.initial_delivery_count).is_some() {
                    for field_name in nulls.drain(..) {
                        state.serialize_field(field_name, &())?;
                    }
                    state
                        .serialize_field("initial-delivery-count", &self.initial_delivery_count)?;
                } else {
                    nulls.push("initial-delivery-count");
                };
                if (&self.max_message_size).is_some() {
                    for field_name in nulls.drain(..) {
                        state.serialize_field(field_name, &())?;
                    }
                    state.serialize_field("max-message-size", &self.max_message_size)?;
                } else {
                    nulls.push("max-message-size");
                };
                if (&self.offered_capabilities).is_some() {
                    for field_name in nulls.drain(..) {
                        state.serialize_field(field_name, &())?;
                    }
                    state.serialize_field("offered-capabilities", &self.offered_capabilities)?;
                } else {
                    nulls.push("offered-capabilities");
                };
                if (&self.desired_capabilities).is_some() {
                    for field_name in nulls.drain(..) {
                        state.serialize_field(field_name, &())?;
                    }
                    state.serialize_field("desired-capabilities", &self.desired_capabilities)?;
                } else {
                    nulls.push("desired-capabilities");
                };
                if (&self.properties).is_some() {
                    for field_name in nulls.drain(..) {
                        state.serialize_field(field_name, &())?;
                    }
                    state.serialize_field("properties", &self.properties)?;
                } else {
                    nulls.push("properties");
                };
                state.end()
            }
        }
    };

    #[test]
    fn test_serialize_deserialize_attach() {
        let attach = Attach {
            name: "sender-link-1".into(),
            handle: 0.into(),
            role: Role::Sender,
            snd_settle_mode: SenderSettleMode::Unsettled,
            rcv_settle_mode: ReceiverSettleMode::First,
            source: Some(Source::default()),
            target: Some("q1".into()),
            unsettled: None,
            incomplete_unsettled: false,
            initial_delivery_count: Some(0),
            max_message_size: Some(0),
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None,
        };
        let buf = to_vec(&attach).unwrap();
        println!("buf len: {:?}", buf.len());
        println!("{:#x?}", buf);
        let deserialized: Attach = from_slice(&buf).unwrap();
        println!("{:?}", deserialized);
    }
}

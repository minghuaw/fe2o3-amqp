use std::collections::BTreeMap;

use serde_amqp::{
    macros::{DeserializeComposite, SerializeComposite},
    primitives::{Boolean, Symbol, ULong, Array},
};

use crate::{
    definitions::{
        DeliveryTag, Fields, Handle, ReceiverSettleMode, Role, SenderSettleMode, SequenceNo,
    },
    messaging::{DeliveryState, Source, TargetArchetype},
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
    ///
    /// This is put inside a `Box` to reduce the variant size of `Performative`.
    /// The use of `Attach` frame and access to this field should be fairly infrequent,
    /// and thus the performance penalty should be negligible (not tested yet).
    pub source: Option<Box<Source>>,

    /// <field name="target" type="*" requires="target"/>
    ///
    /// If no target is specified on an incoming link, then there is no target currently
    /// attached to the link. A link with no target will never permit incoming messages.
    ///
    /// This is put inside a `Box` to reduce the variant size of `Performative`.
    /// The use of `Attach` frame and access to this field should be fairly infrequent,
    /// and thus the performance penalty should be negligible (not tested yet).
    pub target: Option<Box<TargetArchetype>>,

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
    pub offered_capabilities: Option<Array<Symbol>>,

    /// <field name="desired-capabilities" type="symbol" multiple="true"/>
    pub desired_capabilities: Option<Array<Symbol>>,

    /// <field name="properties" type="fields"/>
    pub properties: Option<Fields>,
}

#[cfg(test)]
mod tests {
    use serde_amqp::{from_slice, to_vec};

    use crate::{
        definitions::{ReceiverSettleMode, Role, SenderSettleMode},
        messaging::Source,
    };

    use super::Attach;

    #[test]
    fn test_serialize_deserialize_attach() {
        let attach = Attach {
            name: "sender-link-1".into(),
            handle: 0.into(),
            role: Role::Sender,
            snd_settle_mode: SenderSettleMode::Unsettled,
            rcv_settle_mode: ReceiverSettleMode::First,
            source: Some(Box::new(Source::default())),
            target: Some(Box::new("q1".into())),
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

    #[test]
    fn test_size_of_attach() {
        use super::*;
        let s = std::mem::size_of::<Attach>();
        println!("Attach {:?}", s);

        let s = std::mem::size_of::<String>();
        println!("name {:?}", s);

        let s = std::mem::size_of::<Handle>();
        println!("handle {:?}", s);

        let s = std::mem::size_of::<Role>();
        println!("role {:?}", s);

        let s = std::mem::size_of::<SenderSettleMode>();
        println!("snd_settle_mode {:?}", s);

        let s = std::mem::size_of::<ReceiverSettleMode>();
        println!("rcv_settle_mode {:?}", s);

        let s = std::mem::size_of::<Option<Box<Source>>>();
        println!("source {:?}", s);

        let s = std::mem::size_of::<Option<Box<TargetArchetype>>>();
        println!("target {:?}", s);

        let s = std::mem::size_of::<Option<BTreeMap<DeliveryTag, DeliveryState>>>();
        println!("unsettled {:?}", s);

        let s = std::mem::size_of::<Boolean>();
        println!("incomplete_unsettled {:?}", s);

        let s = std::mem::size_of::<Option<SequenceNo>>();
        println!("initial_delivery_count {:?}", s);

        let s = std::mem::size_of::<Option<ULong>>();
        println!("max_message_size {:?}", s);

        let s = std::mem::size_of::<Option<Vec<Symbol>>>();
        println!("offered_capabilities {:?}", s);

        let s = std::mem::size_of::<Option<Vec<Symbol>>>();
        println!("desired_capabilities {:?}", s);

        let s = std::mem::size_of::<Option<Fields>>();
        println!("properties {:?}", s);
    }
}

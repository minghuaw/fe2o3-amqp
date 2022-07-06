//! Implementation of message properties

use serde_amqp::{
    primitives::{Binary, Symbol, Timestamp},
    DeserializeComposite, SerializeComposite,
};

use crate::definitions::SequenceNo;

use super::{Address, MessageId};

/// 3.2.4 Properties
/// Immutable properties of the message.
/// <type name="properties" class="composite" source="list" provides="section">
///     <descriptor name="amqp:properties:list" code="0x00000000:0x00000073"/>
/// </type>
#[derive(Debug, Clone, SerializeComposite, DeserializeComposite)]
#[amqp_contract(
    name = "amqp:properties:list",
    code = 0x0000_0000_0000_0073,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Properties {
    /// <field name="message-id" type="*" requires="message-id"/>
    pub message_id: Option<MessageId>,

    /// <field name="user-id" type="binary"/>
    pub user_id: Option<Binary>,

    /// <field name="to" type="*" requires="address"/>
    pub to: Option<Address>,

    /// <field name="subject" type="string"/>
    pub subject: Option<String>,

    /// <field name="reply-to" type="*" requires="address"/>
    pub reply_to: Option<Address>,

    /// <field name="correlation-id" type="*" requires="message-id"/>
    pub correlation_id: Option<MessageId>,

    /// <field name="content-type" type="symbol"/>
    pub content_type: Option<Symbol>,

    /// <field name="content-encoding" type="symbol"/>
    pub content_encoding: Option<Symbol>,

    /// <field name="absolute-expiry-time" type="timestamp"/>
    pub absolute_expiry_time: Option<Timestamp>,

    /// <field name="creation-time" type="timestamp"/>
    pub creation_time: Option<Timestamp>,

    /// <field name="group-id" type="string"/>
    pub group_id: Option<String>,

    /// <field name="group-sequence" type="sequence-no"/>
    pub group_sequence: Option<SequenceNo>,

    /// <field name="reply-to-group-id" type="string"/>
    pub reply_to_group_id: Option<String>,
}

/// Every field is default to `None`
impl Default for Properties {
    fn default() -> Self {
        Self::new()
    }
}

impl Properties {
    /// Creates a new instance of [`Properties`]
    pub fn new() -> Self {
        Self {
            message_id: None,
            user_id: None,
            to: None,
            subject: None,
            reply_to: None,
            correlation_id: None,
            content_type: None,
            content_encoding: None,
            absolute_expiry_time: None,
            creation_time: None,
            group_id: None,
            group_sequence: None,
            reply_to_group_id: None,
        }
    }

    /// Creates a new builder for [`Properties`]
    pub fn builder() -> Builder {
        Builder::new()
    }
}

/// Builder for [`Properties`]
#[derive(Debug, Clone)]
pub struct Builder {
    inner: Properties,
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    /// Creates a new builder for [`Properties`]
    pub fn new() -> Self {
        Self {
            inner: Properties::new(),
        }
    }

    /// Set the "message-id" field
    pub fn message_id(mut self, message_id: impl Into<MessageId>) -> Self {
        self.inner.message_id = Some(message_id.into());
        self
    }

    /// Set the "user-id" field
    pub fn user_id(mut self, user_id: impl Into<Binary>) -> Self {
        self.inner.user_id = Some(user_id.into());
        self
    }

    /// Set the "to" field
    pub fn to(mut self, to: impl Into<Address>) -> Self {
        self.inner.to = Some(to.into());
        self
    }

    /// Set the "subject" field
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.inner.subject = Some(subject.into());
        self
    }

    /// Set the "reply-to" field
    pub fn reply_to(mut self, reply_to: impl Into<Address>) -> Self {
        self.inner.reply_to = Some(reply_to.into());
        self
    }

    /// Set the "correlation-id" field
    pub fn correlation_id(mut self, correlation_id: impl Into<MessageId>) -> Self {
        self.inner.correlation_id = Some(correlation_id.into());
        self
    }

    /// Set the "content-type" field
    pub fn content_type(mut self, content_type: impl Into<Symbol>) -> Self {
        self.inner.content_type = Some(content_type.into());
        self
    }

    /// Set the "content-encoding" field
    pub fn content_encoding(mut self, content_encoding: impl Into<Symbol>) -> Self {
        self.inner.content_encoding = Some(content_encoding.into());
        self
    }

    /// Set the "absolute-expiry-time" field
    pub fn absolute_expiry_time(
        mut self,
        absolute_expiry_time: impl Into<Option<Timestamp>>,
    ) -> Self {
        self.inner.absolute_expiry_time = absolute_expiry_time.into();
        self
    }

    /// Set the "creation-time" field
    pub fn creation_time(mut self, creation_time: impl Into<Option<Timestamp>>) -> Self {
        self.inner.creation_time = creation_time.into();
        self
    }

    /// Set the "group-id" field
    pub fn group_id(mut self, group_id: impl Into<Option<String>>) -> Self {
        self.inner.group_id = group_id.into();
        self
    }

    /// Set the "group-sequence" field
    pub fn group_sequence(mut self, group_sequence: impl Into<Option<SequenceNo>>) -> Self {
        self.inner.group_sequence = group_sequence.into();
        self
    }

    /// Set the "reply-to-group-id" field
    pub fn reply_to_group_id(mut self, reply_to_group_id: impl Into<Option<String>>) -> Self {
        self.inner.reply_to_group_id = reply_to_group_id.into();
        self
    }

    /// Build the [`Properties`]
    pub fn build(self) -> Properties {
        self.inner
    }
}

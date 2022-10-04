//! Implementation of message header

use serde_amqp::{DeserializeComposite, SerializeComposite, primitives::{Boolean, UInt}};

use crate::definitions::Milliseconds;

use super::Priority;


/// 3.2.1 Header
/// Transport headers for a message.
/// <type name="header" class="composite" source="list" provides="section">
///     <descriptor name="amqp:header:list" code="0x00000000:0x00000070"/>
/// </type>
#[derive(Debug, Clone, Default, DeserializeComposite, SerializeComposite)]
#[amqp_contract(
    name = "amqp:header:list",
    code = 0x0000_0000_0000_0070,
    encoding = "list",
    rename_all = "kebab-case"
)]
pub struct Header {
    /// <field name="durable" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub durable: Boolean,

    /// <field name="priority" type="ubyte" default="4"/>
    #[amqp_contract(default)]
    pub priority: Priority,

    /// <field name="ttl" type="milliseconds"/>
    pub ttl: Option<Milliseconds>,

    /// <field name="first-acquirer" type="boolean" default="false"/>
    #[amqp_contract(default)]
    pub first_acquirer: Boolean,

    /// <field name="delivery-count" type="uint" default="0"/>
    #[amqp_contract(default)]
    pub delivery_count: UInt,
}

impl Header {
    /// Creates a builder for header
    pub fn builder() -> Builder {
        Default::default()
    }
}

/// Builder for [`Header`]
#[derive(Debug, Default, Clone)]
pub struct Builder {
    inner: Header
}

impl Builder {
    /// Set the `durable` field of [`Header`]
    pub fn durable(mut self, value: Boolean) -> Self {
        self.inner.durable = value;
        self
    }

    /// Set the `priority` field of [`Header`]
    pub fn priority(mut self, value: impl Into<Priority>) -> Self {
        self.inner.priority = value.into();
        self
    }

    /// Set the `ttl` field of [`Header`]
    pub fn ttl(mut self, value: impl Into<Option<Milliseconds>>) -> Self {
        self.inner.ttl = value.into();
        self
    }

    /// Set the `first_acquirer` field of [`Header`]
    pub fn first_acquirer(mut self, value: Boolean) -> Self {
        self.inner.first_acquirer = value;
        self
    }

    /// Set teh `delivery_count` field of [`Header`]
    pub fn delivery_count(mut self, value: UInt) -> Self {
        self.inner.delivery_count = value;
        self
    }

    /// Builds the [`Header`]
    pub fn build(self) -> Header {
        self.inner
    }
}

impl From<Builder> for Header {
    fn from(builder: Builder) -> Self {
        builder.build()
    }
}
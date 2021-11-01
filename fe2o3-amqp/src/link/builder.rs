use std::marker::PhantomData;

use fe2o3_amqp_types::{definitions::{Fields, ReceiverSettleMode, SenderSettleMode, SequenceNo}, messaging::{Source, Target}, primitives::{Symbol, ULong}};

use crate::connection::builder::DEFAULT_OUTGOING_BUFFER_SIZE;

use super::role;

/// Type state for link::builder::Builder;
pub struct WithoutName;

/// Type state for link::builder::Builder;
pub struct WithName;

pub struct Builder<Role, NameState> {
    pub name: String,
    pub snd_settle_mode: SenderSettleMode,
    pub rcv_settle_mode: ReceiverSettleMode,
    pub source: Option<Source>,
    pub target: Option<Target>,

    /// This MUST NOT be null if role is sender,
    /// and it is ignored if the role is receiver.
    /// See subsection 2.6.7.
    pub initial_delivery_count: SequenceNo,

    pub max_message_size: Option<ULong>,
    pub offered_capabilities: Option<Vec<Symbol>>,
    pub desired_capabilities: Option<Vec<Symbol>>,
    pub properties: Option<Fields>,

    pub buffer_size: usize,

    // Type state markers
    role: PhantomData<Role>,
    name_state: PhantomData<NameState>,
}

impl<Role> Builder<Role, WithoutName> {
    pub fn new() -> Self {
        Self {
            name: Default::default(),
            snd_settle_mode: Default::default(),
            rcv_settle_mode: Default::default(),
            source: Default::default(),
            target: Default::default(),
            initial_delivery_count: Default::default(),
            max_message_size: Default::default(),
            offered_capabilities: Default::default(),
            desired_capabilities: Default::default(),
            properties: Default::default(),
            
            buffer_size: DEFAULT_OUTGOING_BUFFER_SIZE,
            role: PhantomData,
            name_state: PhantomData,
        }
    }

    pub fn name(self, name: impl Into<String>) -> Builder<Role, WithName> {
        todo!()
    }
}

impl<Role> Builder<Role, WithName> {
    pub fn name(&mut self, name: impl Into<String>) -> &mut Self {
        todo!()
    }
}

impl<Role, NameState> Builder<Role, NameState> {
    pub fn sender(self) -> Builder<role::Sender, NameState> {
        Builder {
            name: self.name,
            snd_settle_mode: self.snd_settle_mode,
            rcv_settle_mode: self.rcv_settle_mode,
            source: self.source,
            target: self.target,
            initial_delivery_count: self.initial_delivery_count,
            max_message_size: self.max_message_size,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,
            buffer_size: self.buffer_size,
            properties: Default::default(),

            role: PhantomData,
            name_state: self.name_state,
        }
    }

    pub fn receiver(self) -> Builder<role::Receiver, NameState> {
        Builder {
            name: self.name,
            snd_settle_mode: self.snd_settle_mode,
            rcv_settle_mode: self.rcv_settle_mode,
            source: self.source,
            target: self.target,
            initial_delivery_count: self.initial_delivery_count,
            max_message_size: self.max_message_size,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,
            buffer_size: self.buffer_size,
            properties: Default::default(),

            role: PhantomData,
            name_state: self.name_state,
        }
    }

    pub fn sender_settle_mode(&mut self, mode: SenderSettleMode) -> &mut Self {
        self.snd_settle_mode = mode;
        self
    }

    pub fn receiver_settle_mode(&mut self, mode: ReceiverSettleMode) -> &mut Self {
        self.rcv_settle_mode = mode;
        self
    }

    pub fn source(&mut self, source: Option<Source>) -> &mut Self {
        self.source = source;
        self
    }

    pub fn target(&mut self, target: Option<Target>) -> &mut Self {
        self.target = target;
        self
    }

    pub fn max_message_size(&mut self, max_size: Option<ULong>) -> &mut Self {
        self.max_message_size = max_size;
        self
    }

    pub fn add_offered_capabilities(&mut self, capability: impl Into<Symbol>) -> &mut Self {
        match &mut self.offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.offered_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    pub fn set_offered_capabilities(&mut self, capabilities: Option<Vec<Symbol>>) -> &mut Self {
        self.offered_capabilities = capabilities;
        self
    }

    pub fn add_desired_capabilities(&mut self, capability: impl Into<Symbol>) -> &mut Self {
        match &mut self.desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.desired_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    pub fn set_desired_capabilities(&mut self, capabilities: Option<Vec<Symbol>>) -> &mut Self {
        self.desired_capabilities = capabilities;
        self
    }

    pub fn properties(&mut self, properties: Option<Fields>) -> &mut Self {
        self.properties = properties;
        self
    }
}

impl<NameState> Builder<role::Sender, NameState> {
    /// This MUST NOT be null if role is sender,
    /// and it is ignored if the role is receiver.
    /// See subsection 2.6.7.
    pub fn initial_delivery_count(&mut self, count: SequenceNo) -> &mut Self {
        self.initial_delivery_count = count;
        self
    }
}

impl<NameState> Builder<role::Receiver, NameState> {

}

impl Builder<role::Sender, WithName> {

}

impl Builder<role::Receiver, WithName> {

}

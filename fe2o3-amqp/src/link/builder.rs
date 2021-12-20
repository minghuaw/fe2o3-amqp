use std::{
    collections::BTreeMap,
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc,
    },
};

use fe2o3_amqp_types::{
    definitions::{Fields, ReceiverSettleMode, SenderSettleMode, SequenceNo, AmqpError, Handle},
    messaging::{Source, Target},
    performatives::{Detach, Attach},
    primitives::{Symbol, ULong},
};
use futures_util::{Sink, SinkExt, Stream};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::{
    connection::builder::DEFAULT_OUTGOING_BUFFER_SIZE,
    link::{
        sender_link::SenderLink, LinkFlowState, LinkFlowStateInner, LinkFrame, LinkHandle,
        LinkIncomingItem, LinkState, Error
    },
    session::SessionHandle,
    util::Constant, endpoint,
};

use super::{role, Receiver, Sender, type_state::Attached};

/// Type state for link::builder::Builder;
pub struct WithoutName;

/// Type state for link::builder::Builder;
pub struct WithName;

/// Type state for link::builder::Builder;
pub struct WithoutTarget;

/// Type state for link::builder::Builder;
pub struct WithTarget;

pub struct Builder<Role, NameState, Addr> {
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
    addr_state: PhantomData<Addr>,
}

impl<Role, Addr> Builder<Role, WithoutName, Addr> {
    pub(crate) fn new() -> Self {
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
            addr_state: PhantomData,
        }
    }

    pub fn name(self, name: impl Into<String>) -> Builder<Role, WithName, Addr> {
        Builder {
            name: name.into(),
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

            role: self.role,
            name_state: PhantomData,
            addr_state: self.addr_state,
        }
    }
}

impl<Role, Addr> Builder<Role, WithName, Addr> {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
}

impl<Role, NameState, Addr> Builder<Role, NameState, Addr> {
    pub fn sender(self) -> Builder<role::Sender, NameState, Addr> {
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
            addr_state: self.addr_state,
        }
    }

    pub fn receiver(self) -> Builder<role::Receiver, NameState, Addr> {
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
            addr_state: self.addr_state,
        }
    }

    pub fn sender_settle_mode(mut self, mode: SenderSettleMode) -> Self {
        self.snd_settle_mode = mode;
        self
    }

    pub fn receiver_settle_mode(mut self, mode: ReceiverSettleMode) -> Self {
        self.rcv_settle_mode = mode;
        self
    }

    pub fn source(mut self, source: impl Into<Source>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn target(self, target: impl Into<Target>) -> Builder<Role, NameState, WithTarget> {
        Builder {
            name: self.name,
            snd_settle_mode: self.snd_settle_mode,
            rcv_settle_mode: self.rcv_settle_mode,
            source: self.source,
            target: Some(target.into()), // setting target
            initial_delivery_count: self.initial_delivery_count,
            max_message_size: self.max_message_size,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,
            buffer_size: self.buffer_size,
            properties: Default::default(),

            role: self.role,
            name_state: self.name_state,
            addr_state: PhantomData,
        }
    }

    pub fn max_message_size(mut self, max_size: impl Into<ULong>) -> Self {
        self.max_message_size = Some(max_size.into());
        self
    }

    pub fn add_offered_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.offered_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.offered_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    pub fn set_offered_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.offered_capabilities = Some(capabilities);
        self
    }

    pub fn add_desired_capabilities(mut self, capability: impl Into<Symbol>) -> Self {
        match &mut self.desired_capabilities {
            Some(capabilities) => capabilities.push(capability.into()),
            None => self.desired_capabilities = Some(vec![capability.into()]),
        }
        self
    }

    pub fn set_desired_capabilities(mut self, capabilities: Vec<Symbol>) -> Self {
        self.desired_capabilities = Some(capabilities);
        self
    }

    pub fn properties(mut self, properties: Fields) -> Self {
        self.properties = Some(properties);
        self
    }
}

impl<NameState, Addr> Builder<role::Sender, NameState, Addr> {
    /// This MUST NOT be null if role is sender,
    /// and it is ignored if the role is receiver.
    /// See subsection 2.6.7.
    pub fn initial_delivery_count(mut self, count: SequenceNo) -> Self {
        self.initial_delivery_count = count;
        self
    }
}

impl<NameState, Addr> Builder<role::Receiver, NameState, Addr> {}

impl Builder<role::Sender, WithName, WithTarget> {
    pub async fn attach(self, session: &mut SessionHandle) -> Result<Sender<Attached>, Error> {
        use crate::endpoint;

        let local_state = LinkState::Unattached;
        let (incoming_tx, mut incoming_rx) = mpsc::channel::<LinkIncomingItem>(self.buffer_size);
        let outgoing = PollSender::new(session.outgoing.clone());

        // Create shared link flow state
        let flow_state_inner = LinkFlowStateInner {
            intial_delivery_count: Constant::new(self.initial_delivery_count),
            delivery_count: AtomicU32::new(self.initial_delivery_count),
            // The link-credit and available variables are initialized to zero.
            link_credit: AtomicU32::new(0),
            avaiable: AtomicU32::new(0),
            // The drain flag is initialized to false.
            drain: AtomicBool::new(false),
            properties: RwLock::new(self.properties),
        };
        let flow_state = Arc::new(LinkFlowState::Sender(flow_state_inner));
        let link_handle = LinkHandle {
            tx: incoming_tx,
            flow_state: flow_state.clone(),
        };

        // Create Link in Session
        let output_handle = session
            .allocate_link(self.name.clone(), link_handle)
            .await?;

        // Get writer to session
        let writer = session.outgoing.clone();

        let max_message_size = match self.max_message_size {
            Some(s) => s,
            None => 0,
        };

        // Create a SenderLink instance
        let mut link = SenderLink {
            local_state,
            name: self.name,
            output_handle: Some(output_handle.clone()),
            input_handle: None,
            snd_settle_mode: self.snd_settle_mode,
            rcv_settle_mode: self.rcv_settle_mode,
            source: self.source, // TODO: how should this field be set?
            target: self.target,
            unsettled: BTreeMap::new(),
            max_message_size,
            offered_capabilities: self.offered_capabilities,
            desired_capabilities: self.desired_capabilities,

            // delivery_count: self.initial_delivery_count,
            // properties: self.properties,
            flow_state,
        };

        // Send an Attach frame
        let mut writer = PollSender::new(writer);
        let mut reader = ReceiverStream::new(incoming_rx);
        let remote_attach = super::do_attach(&mut link, &mut writer, &mut reader).await?;

        // Note that if the application chooses not to create a terminus,
        // the session endpoint will still create a link endpoint and issue
        // an attach indicating that the link endpoint has no associated
        // local terminus. In this case, the session endpoint MUST immediately
        // detach the newly created link endpoint.
        match remote_attach.target.is_some() {
            true => {
                if let Err(_) = endpoint::Link::on_incoming_attach(&mut link, remote_attach).await {
                    // Should any error happen handling remote 
                    todo!()
                }
            },
            false => {
                // If no target is supplied with the remote attach frame,
                // an immediate detach should be expected
                expect_detach_then_detach(&mut writer, &mut reader, output_handle).await?;
            }
        }

        // Attach completed, return Sender
        let sender = Sender::<Attached> {
            link,
            outgoing,
            incoming: reader,
            marker: PhantomData,
        };
        Ok(sender)
    }
}

impl Builder<role::Receiver, WithName, WithTarget> {
    pub async fn attach(&self, session: &mut SessionHandle) -> Result<Receiver, Error> {
        todo!()
    }
}

async fn expect_detach_then_detach<W, R>(
    writer: &mut W,
    reader: &mut R, 
    output_handle: Handle,
) -> Result<(), Error> 
where 
    W: Sink<LinkFrame, Error = mpsc::error::SendError<LinkFrame>> + Unpin,
    R: Stream<Item = LinkFrame> + Send + Unpin,
{
    use futures_util::StreamExt;

    let frame = reader
        .next()
        .await
        .ok_or_else(|| Error::AmqpError {
            condition: AmqpError::IllegalState,
            description: Some("Expecting remote detach frame".to_string())
        })?;
    let _remote_detach = match frame {
        LinkFrame::Detach(detach) => detach,
        _ => return Err(Error::AmqpError {
            condition: AmqpError::IllegalState,
            description: Some("Expecting remote detach frame".to_string())
        }),
    };

    let detach = Detach {
        handle: output_handle,
        closed: true,
        error: None,
    };
    let frame = LinkFrame::Detach(detach);
    writer.send(frame).await?;
    Ok(())
}

use fe2o3_amqp_types::messaging::message::__private::Deserializable;
use serde_amqp::format_code::EncodingCodes;

use super::*;

const DESCRIBED_TYPE: u8 = EncodingCodes::DescribedType as u8;
const SMALL_ULONG_TYPE: u8 = EncodingCodes::SmallUlong as u8;
const ULONG_TYPE: u8 = EncodingCodes::ULong as u8;
const HEADER_CODE: u8 = 0x70;
const DELIV_ANNOT_CODE: u8 = 0x71;
const MSG_ANNOT_CODE: u8 = 0x72;
const PROP_CODE: u8 = 0x73;
const APP_PROP_CODE: u8 = 0x74;
const DATA_CODE: u8 = 0x75;
const AMQP_SEQ_CODE: u8 = 0x76;
const AMQP_VAL_CODE: u8 = 0x77;
const FOOTER_CODE: u8 = 0x78;

#[async_trait]
impl<Tar> endpoint::ReceiverLink for ReceiverLink<Tar>
where
    Tar: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
{
    type FlowError = FlowError;
    type TransferError = ReceiverTransferError;
    type DispositionError = DispositionError;

    /// Set and send flow state
    async fn send_flow(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        link_credit: Option<u32>,
        drain: Option<bool>,
        echo: bool,
    ) -> Result<(), Self::FlowError> {
        // self.error_if_closed().map_err(|_| Error::NotAttached)?;

        let handle = self
            .output_handle
            .clone()
            .ok_or(Self::FlowError::IllegalState)?
            .into();

        let flow = match (link_credit, drain) {
            (Some(link_credit), Some(drain)) => {
                let mut writer = self.flow_state.lock.write().await;
                writer.link_credit = link_credit;
                writer.drain = drain;
                LinkFlow {
                    handle,
                    // TODO: "last known value"???
                    // When the flow state is being sent from the receiver endpoint to the sender
                    // endpoint this field MUST be set to the last known value of the corresponding
                    // sending endpoint.
                    delivery_count: Some(writer.delivery_count),
                    link_credit: Some(link_credit),
                    // The receiver sets this to the last known value seen from the sender
                    // available: Some(writer.available),
                    available: None,
                    drain,
                    echo,
                    properties: writer.properties.clone(),
                }
            }
            (Some(link_credit), None) => {
                let mut writer = self.flow_state.lock.write().await;
                writer.link_credit = link_credit;
                LinkFlow {
                    handle,
                    // TODO: "last known value"???
                    // When the flow state is being sent from the receiver endpoint to the sender
                    // endpoint this field MUST be set to the last known value of the corresponding
                    // sending endpoint.
                    delivery_count: Some(writer.delivery_count),
                    link_credit: Some(link_credit),
                    // The receiver sets this to the last known value seen from the sender
                    // available: Some(writer.available),
                    available: None,
                    drain: writer.drain,
                    echo,
                    properties: writer.properties.clone(),
                }
            }
            (None, Some(drain)) => {
                let mut writer = self.flow_state.lock.write().await;
                writer.drain = drain;
                LinkFlow {
                    handle,
                    // TODO: "last known value"???
                    // When the flow state is being sent from the receiver endpoint to the sender
                    // endpoint this field MUST be set to the last known value of the corresponding
                    // sending endpoint.
                    delivery_count: Some(writer.delivery_count),
                    link_credit: Some(writer.link_credit),
                    // The receiver sets this to the last known value seen from the sender
                    // available: Some(writer.available),
                    available: None,
                    drain,
                    echo,
                    properties: writer.properties.clone(),
                }
            }
            (None, None) => {
                let reader = self.flow_state.lock.read().await;
                LinkFlow {
                    handle,
                    // TODO: "last known value"???
                    // When the flow state is being sent from the receiver endpoint to the sender
                    // endpoint this field MUST be set to the last known value of the corresponding
                    // sending endpoint.
                    delivery_count: Some(reader.delivery_count),
                    link_credit: Some(reader.link_credit),
                    // The receiver sets this to the last known value seen from the sender
                    // available: Some(writer.available),
                    available: None,
                    drain: reader.drain,
                    echo,
                    properties: reader.properties.clone(),
                }
            }
        };
        writer
            .send(LinkFrame::Flow(flow))
            .await
            .map_err(|_| Self::FlowError::IllegalSessionState)
    }

    async fn on_incomplete_transfer(
        &mut self,
        delivery_tag: DeliveryTag,
        section_number: u32,
        section_offset: u64,
    ) {
        // link-credit is defined as
        // "The current maximum number of messages that can be handled
        // at the receiver endpoint of the link"
        // So there is no need to decrement the link-credit on incomplete delivery

        let state = DeliveryState::Received(Received {
            section_number,
            section_offset,
        });

        {
            let mut lock = self.unsettled.write().await;
            // The same key may be writter multiple times
            let _ = lock.insert(delivery_tag, state);
        }
    }

    async fn on_complete_transfer<T>(
        &mut self,
        transfer: Transfer,
        payload: Payload,
        // section_number: u32,
        // section_offset: u64,
    ) -> Result<
        (
            Delivery<T>,
            Option<(DeliveryNumber, DeliveryTag, DeliveryState)>,
        ),
        Self::TransferError,
    >
    where
        T: for<'de> serde::Deserialize<'de> + Send,
    {
        // ReceiverFlowState will not wait until link credit is available.
        // Will return with an error if there is not enough link credit.
        // TODO: The receiver should then detach with error
        self.flow_state.consume(1).await?;

        // Upon receiving the transfer, the receiving link endpoint (receiver)
        // will create an entry in its own unsettled map and make the transferred
        // message data available to the application to process.

        // This only takes care of whether the message is considered
        // sett
        let settled_by_sender = transfer.settled.unwrap_or(false);
        let delivery_id = transfer
            .delivery_id
            .ok_or(Self::TransferError::DeliveryIdIsNone)?;
        let delivery_tag = transfer
            .delivery_tag
            .ok_or(Self::TransferError::DeliveryTagIsNone)?;

        let (message, delivery_state) = if settled_by_sender {
            // If the message is pre-settled, there is no need to
            // add to the unsettled map and no need to reply to the Sender
            let message: Deserializable<Message<T>> = from_reader(payload.reader())
                .map_err(|_| Self::TransferError::MessageDecodeError)?;
            (message.0, None)
        } else {
            // If the message is being sent settled by the sender, the value of this
            // field is ignored.
            let mode = if let Some(mode) = &transfer.rcv_settle_mode {
                // If the negotiated link value is first, then it is illegal to set this
                // field to second.
                if let ReceiverSettleMode::First = &self.rcv_settle_mode {
                    if let ReceiverSettleMode::Second = mode {
                        return Err(Self::TransferError::IllegalRcvSettleModeInTransfer);
                    }
                }
                mode
            } else {
                // If not set, this value is defaulted to the value negotiated on link
                // attach.
                &self.rcv_settle_mode
            };

            match mode {
                // If first, this indicates that the receiver MUST settle the delivery
                // once it has arrived without waiting for the sender to settle first.
                ReceiverSettleMode::First => {
                    // Spontaneously settle the message with an Accept
                    // let reader = IoReader::new(payload.reader());
                    // let deserializer = Deserializer::new(reader);
                    // let message: Message<T> = Message::<T>::deserialize(&mut deserializer)?;
                    let message: Deserializable<Message<T>> = from_reader(payload.reader())
                        .map_err(|_| Self::TransferError::MessageDecodeError)?;

                    (message.0, Some(DeliveryState::Accepted(Accepted {})))
                }
                // If second, this indicates that the receiver MUST NOT settle until
                // sending its disposition to the sender and receiving a settled
                // disposition from the sender.
                ReceiverSettleMode::Second => {
                    // Add to unsettled map
                    let section_offset = rfind_offset_of_complete_message(payload.as_ref())
                        .ok_or(Self::TransferError::MessageDecodeError)?;
                    let message: Deserializable<Message<T>> = from_reader(payload.reader())
                        .map_err(|_| Self::TransferError::MessageDecodeError)?;
                    let message = message.0;
                    let section_number = message.sections();

                    let state = DeliveryState::Received(Received {
                        section_number, // What is section number?
                        section_offset,
                    });

                    // Insert into local unsettled map with Received state
                    // Mode Second doesn't automatically send back a disposition
                    // (ie. thus doesn't call `link.dispose()`) and thus need to manually
                    // set the delivery state
                    {
                        let mut lock = self.unsettled.write().await;
                        // There may be records of incomplete delivery
                        let _ = lock.insert(delivery_tag.clone(), state);
                    }

                    // Mode Second requires user to explicitly acknowledge the delivery
                    // with Accept, Release, ...
                    (message, None)
                }
            }
        };

        let link_output_handle = self
            .output_handle
            .clone()
            .ok_or(ReceiverTransferError::IllegalState)?
            .into();

        let delivery = Delivery {
            link_output_handle,
            delivery_id,
            delivery_tag: delivery_tag.clone(),
            message,
        };

        Ok((
            delivery,
            delivery_state.map(|state| (delivery_id, delivery_tag, state)),
        ))
    }

    async fn dispose(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        delivery_id: DeliveryNumber,
        delivery_tag: DeliveryTag,
        // settled: bool,
        state: DeliveryState,
        batchable: bool,
    ) -> Result<(), Self::DispositionError> {
        // self.error_if_closed().map_err(|_| Error::IllegalState)?;

        let settled = match self.rcv_settle_mode {
            ReceiverSettleMode::First => {
                // If first, this indicates that the receiver MUST settle
                // the delivery once it has arrived without waiting
                // for the sender to settle first.

                // The delivery is not inserted into unsettled map if in First mode
                true
            }
            ReceiverSettleMode::Second => {
                // If second, this indicates that the receiver MUST NOT settle until sending
                // its disposition to the sender and receiving a settled disposition from
                // the sender.
                let mut lock = self.unsettled.write().await;
                // If the key is present in the map, the old value will be returned, which
                // we don't really need
                let _ = lock.insert(delivery_tag.clone(), state.clone());
                false
            }
        };

        let disposition = Disposition {
            role: Role::Receiver,
            first: delivery_id,
            last: None,
            settled,
            state: Some(state),
            batchable,
        };
        let frame = LinkFrame::Disposition(disposition);
        writer
            .send(frame)
            .await
            .map_err(|_| Self::DispositionError::IllegalSessionState)?;

        // This is a unit enum, clone should be really cheap
        Ok(())
    }
}

/// Finds offset of a complete message
fn rfind_offset_of_complete_message(bytes: &[u8]) -> Option<u64> {
    // For a complete message, the only need is to check Footer or Body

    let len = bytes.len();
    let mut iter = bytes
        .iter()
        .zip(bytes.iter().skip(1).zip(bytes.iter().skip(2)));

    iter.rposition(|(&b0, (&b1, &b2))| {
        matches!(
            (b0, b1, b2),
            (DESCRIBED_TYPE, SMALL_ULONG_TYPE, DATA_CODE)
            | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, AMQP_SEQ_CODE)
            | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, AMQP_VAL_CODE)
            | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, FOOTER_CODE)
            // Some implementation may use ULong for all u64 numbers
            | (DESCRIBED_TYPE, ULONG_TYPE, DATA_CODE)
            | (DESCRIBED_TYPE, ULONG_TYPE, AMQP_SEQ_CODE)
            | (DESCRIBED_TYPE, ULONG_TYPE, AMQP_VAL_CODE)
            | (DESCRIBED_TYPE, ULONG_TYPE, FOOTER_CODE)
        )
    })
    .map(|val| (len - val) as u64)
}

/// Count number of sections in encoded message
pub(crate) fn section_number_and_offset(bytes: &[u8]) -> (u32, u64) {
    let mut last_pos = 0;
    let mut section_numbers = 0;

    let iter = bytes
        .iter()
        .zip(bytes.iter().skip(1).zip(bytes.iter().skip(2)));
    for (i, (&b0, (&b1, &b2))) in iter.enumerate() {
        match (b0, b1, b2) {
            (DESCRIBED_TYPE, SMALL_ULONG_TYPE, HEADER_CODE)
            | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, DELIV_ANNOT_CODE)
            | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, MSG_ANNOT_CODE)
            | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, PROP_CODE)
            | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, APP_PROP_CODE)
            | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, DATA_CODE)
            | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, AMQP_SEQ_CODE)
            | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, AMQP_VAL_CODE)
            | (DESCRIBED_TYPE, SMALL_ULONG_TYPE, FOOTER_CODE)
            // Some implementation may use ULong for all u64 numbers
            | (DESCRIBED_TYPE, ULONG_TYPE, HEADER_CODE)
            | (DESCRIBED_TYPE, ULONG_TYPE, DELIV_ANNOT_CODE)
            | (DESCRIBED_TYPE, ULONG_TYPE, MSG_ANNOT_CODE)
            | (DESCRIBED_TYPE, ULONG_TYPE, PROP_CODE)
            | (DESCRIBED_TYPE, ULONG_TYPE, APP_PROP_CODE)
            | (DESCRIBED_TYPE, ULONG_TYPE, DATA_CODE)
            | (DESCRIBED_TYPE, ULONG_TYPE, AMQP_SEQ_CODE)
            | (DESCRIBED_TYPE, ULONG_TYPE, AMQP_VAL_CODE)
            | (DESCRIBED_TYPE, ULONG_TYPE, FOOTER_CODE) => {
                section_numbers += 1;
                last_pos = i;
            },
            _ => {}
        }
    }

    let offset = bytes.len() - last_pos;
    (section_numbers, offset as u64)
}

impl ReceiverLink<Target> {
    /// Set and send flow state
    #[cfg(feature = "transaction")]
    pub(crate) fn blocking_send_flow(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        link_credit: Option<u32>,
        drain: Option<bool>,
        echo: bool,
    ) -> Result<(), FlowError> {
        // self.error_if_closed().map_err(|e| link::Error::Local(e))?;

        let handle = self
            .output_handle
            .clone()
            .ok_or(FlowError::IllegalState)?
            .into();

        let flow = match (link_credit, drain) {
            (Some(link_credit), Some(drain)) => {
                let mut writer = self.flow_state.lock.blocking_write();
                writer.link_credit = link_credit;
                writer.drain = drain;
                LinkFlow {
                    handle,
                    // TODO: "last known value"???
                    // When the flow state is being sent from the receiver endpoint to the sender
                    // endpoint this field MUST be set to the last known value of the corresponding
                    // sending endpoint.
                    delivery_count: Some(writer.delivery_count.clone()),
                    link_credit: Some(link_credit),
                    // The receiver sets this to the last known value seen from the sender
                    // available: Some(writer.available),
                    available: None,
                    drain,
                    echo,
                    properties: writer.properties.clone(),
                }
            }
            (Some(link_credit), None) => {
                let mut writer = self.flow_state.lock.blocking_write();
                writer.link_credit = link_credit;
                LinkFlow {
                    handle,
                    // TODO: "last known value"???
                    // When the flow state is being sent from the receiver endpoint to the sender
                    // endpoint this field MUST be set to the last known value of the corresponding
                    // sending endpoint.
                    delivery_count: Some(writer.delivery_count.clone()),
                    link_credit: Some(link_credit),
                    // The receiver sets this to the last known value seen from the sender
                    // available: Some(writer.available),
                    available: None,
                    drain: writer.drain,
                    echo,
                    properties: writer.properties.clone(),
                }
            }
            (None, Some(drain)) => {
                let mut writer = self.flow_state.lock.blocking_write();
                writer.drain = drain;
                LinkFlow {
                    handle,
                    // TODO: "last known value"???
                    // When the flow state is being sent from the receiver endpoint to the sender
                    // endpoint this field MUST be set to the last known value of the corresponding
                    // sending endpoint.
                    delivery_count: Some(writer.delivery_count.clone()),
                    link_credit: Some(writer.link_credit),
                    // The receiver sets this to the last known value seen from the sender
                    // available: Some(writer.available),
                    available: None,
                    drain,
                    echo,
                    properties: writer.properties.clone(),
                }
            }
            (None, None) => {
                let reader = self.flow_state.lock.blocking_read();
                LinkFlow {
                    handle,
                    // TODO: "last known value"???
                    // When the flow state is being sent from the receiver endpoint to the sender
                    // endpoint this field MUST be set to the last known value of the corresponding
                    // sending endpoint.
                    delivery_count: Some(reader.delivery_count.clone()),
                    link_credit: Some(reader.link_credit),
                    // The receiver sets this to the last known value seen from the sender
                    // available: Some(writer.available),
                    available: None,
                    drain: reader.drain,
                    echo,
                    properties: reader.properties.clone(),
                }
            }
        };
        writer
            .blocking_send(LinkFrame::Flow(flow))
            .map_err(|_| FlowError::IllegalSessionState)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use fe2o3_amqp_types::{
        messaging::{
            message::{Body, __private::Serializable},
            AmqpValue, DeliveryAnnotations, Header, Message, MessageAnnotations,
        },
        primitives::Value,
    };
    use serde_amqp::to_vec;

    use crate::link::receiver_link::section_number_and_offset;

    #[test]
    fn test_section_numbers() {
        let message = Message {
            header: Some(Header {
                durable: true,
                ..Default::default()
            }),
            // header: None,
            delivery_annotations: Some(DeliveryAnnotations(BTreeMap::new())),
            // delivery_annotations: None,
            message_annotations: Some(MessageAnnotations(BTreeMap::new())),
            // message_annotations: None,
            properties: None,
            application_properties: None,
            body: Body::Value(AmqpValue(Value::Bool(true))),
            footer: None,
        };
        // let mut buf = Vec::new();
        // let mut serializer = serde_amqp::ser::Serializer::new(&mut buf);
        // message.serialize(&mut serializer).unwrap();
        let buf = to_vec(&Serializable(message)).unwrap();
        let (nums, offset) = section_number_and_offset(&buf);
        println!("{:?}, {:?}", nums, offset);
    }
}


#[async_trait]
impl<T> endpoint::LinkAttach for ReceiverLink<T>
where
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
{
    type AttachError = ReceiverAttachError;

    async fn on_incoming_attach(&mut self, remote_attach: Attach) -> Result<(), Self::AttachError> {
        use self::source::VerifySource;

        match self.local_state {
            LinkState::AttachSent => self.local_state = LinkState::Attached,
            LinkState::Unattached => self.local_state = LinkState::AttachReceived,
            LinkState::Detached => self.local_state = LinkState::AttachReceived, // re-attaching
            _ => return Err(ReceiverAttachError::IllegalState),
        };

        self.input_handle = Some(InputHandle::from(remote_attach.handle));

        // In this case, the sender is considered to hold the authoritative version of the
        // version of the source properties
        let remote_source = remote_attach
            .source
            .ok_or(ReceiverAttachError::IncomingSourceIsNone)?;
        if let Some(local_source) = &self.source {
            local_source.verify_as_receiver(&remote_source)?;
        }
        self.source = Some(*remote_source);

        // The receiver SHOULD respect the sender’s desired settlement mode if the sender
        // initiates the attach exchange and the receiver supports the desired mode.
        self.snd_settle_mode = remote_attach.snd_settle_mode;

        // The delivery-count is initialized by the sender when a link endpoint is
        // created, and is incremented whenever a message is sent
        let initial_delivery_count = remote_attach
            .initial_delivery_count
            .ok_or(ReceiverAttachError::InitialDeliveryCountIsNone)?;

        let target = remote_attach
            .target
            .map(|t| T::try_from(*t))
            .transpose()
            .map_err(|_| ReceiverAttachError::CoordinatorIsNotImplemented)?;

        // TODO: **the receiver is considered to hold the authoritative version of the target properties**,
        // Is this verification necessary?
        match (&self.target, &target) {
            (Some(local_target), Some(remote_target)) => {
                local_target.verify_as_receiver(remote_target)?
            }
            (_, None) => return Err(ReceiverAttachError::IncomingTargetIsNone),
            _ => {}
        }

        self.max_message_size =
            get_max_message_size(self.max_message_size, remote_attach.max_message_size);

        self.flow_state
            .as_ref()
            .initial_delivery_count_mut(|_| initial_delivery_count)
            .await;
        self.flow_state
            .as_ref()
            .delivery_count_mut(|_| initial_delivery_count)
            .await;

        Ok(())
    }

    async fn send_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
    ) -> Result<(), Self::AttachError> {
        self.send_attach_inner(writer).await?;
        Ok(())
    }
}


impl<T> endpoint::Link for ReceiverLink<T>
where
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
{
    fn role() -> Role {
        Role::Receiver
    }
}

#[async_trait]
impl<T> endpoint::LinkExt for ReceiverLink<T>
where
    T: Into<TargetArchetype> + TryFrom<TargetArchetype> + VerifyTargetArchetype + Clone + Send,
{
    type FlowState = ReceiverFlowState;
    type Unsettled = Arc<RwLock<UnsettledMap<DeliveryState>>>;
    type Target = T;

    fn name(&self) -> &str {
        &self.name
    }

    fn output_handle(&self) -> &Option<OutputHandle> {
        &self.output_handle
    }

    fn output_handle_mut(&mut self) -> &mut Option<OutputHandle> {
        &mut self.output_handle
    }

    fn flow_state(&self) -> &Self::FlowState {
        &self.flow_state
    }

    fn unsettled(&self) -> &Self::Unsettled {
        &self.unsettled
    }

    fn rcv_settle_mode(&self) -> &ReceiverSettleMode {
        &self.rcv_settle_mode
    }

    fn target(&self) -> &Option<Self::Target> {
        &self.target
    }

    async fn negotiate_attach(
        &mut self,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
    ) -> Result<(), ReceiverAttachError> {
        // Send out local attach
        self.send_attach(writer).await?;

        // Wait for remote attach
        let remote_attach = match reader
            .recv()
            .await
            .ok_or(ReceiverAttachError::IllegalSessionState)?
        {
            LinkFrame::Attach(attach) => attach,
            _ => return Err(ReceiverAttachError::NonAttachFrameReceived),
        };

        self.on_incoming_attach(remote_attach).await
    }

    async fn handle_attach_error(
        &mut self,
        attach_error: ReceiverAttachError,
        writer: &mpsc::Sender<LinkFrame>,
        reader: &mut mpsc::Receiver<LinkFrame>,
        session: &mpsc::Sender<SessionControl>,
    ) -> ReceiverAttachError {
        match attach_error {
            ReceiverAttachError::IllegalSessionState
            | ReceiverAttachError::IllegalState
            | ReceiverAttachError::NonAttachFrameReceived
            | ReceiverAttachError::ExpectImmediateDetach 
            | ReceiverAttachError::RemoteClosedWithError(_) => attach_error,

            ReceiverAttachError::DuplicatedLinkName => {
                let error = definitions::Error::new(
                    SessionError::HandleInUse,
                    "Link name is in use".to_string(),
                    None,
                );
                session
                    .send(SessionControl::End(Some(error)))
                    .await
                    .map(|_| attach_error)
                    .unwrap_or(ReceiverAttachError::IllegalSessionState)
            }
            ReceiverAttachError::IncomingSourceIsNone
            | ReceiverAttachError::IncomingTargetIsNone => {
                // Just send detach immediately
                let err = self
                    .send_detach(writer, true, None)
                    .await
                    .map(|_| attach_error)
                    .unwrap_or(ReceiverAttachError::IllegalSessionState);
                match reader.recv().await {
                    Some(LinkFrame::Detach(remote_detach)) => {
                        match self.on_incoming_detach(remote_detach).await {
                            Ok(_) => return err,
                            Err(detach_error) => {
                                return detach_error.try_into().unwrap_or_else(|_| err)
                            }
                        }
                    }
                    Some(_) => ReceiverAttachError::NonAttachFrameReceived,
                    None => ReceiverAttachError::IllegalSessionState,
                }
            }

            ReceiverAttachError::CoordinatorIsNotImplemented
            | ReceiverAttachError::InitialDeliveryCountIsNone
            | ReceiverAttachError::SourceAddressIsNoneWhenDynamicIsTrue
            | ReceiverAttachError::TargetAddressIsSomeWhenDynamicIsTrue
            | ReceiverAttachError::DynamicNodePropertiesIsSomeWhenDynamicIsFalse => {
                match (&attach_error).try_into() {
                    Ok(error) => {
                        match self.send_detach(writer, true, Some(error)).await {
                            Ok(_) => match reader.recv().await {
                                Some(LinkFrame::Detach(remote_detach)) => {
                                    let _ = self.on_incoming_detach(remote_detach).await; // FIXME: how to handle this?
                                    attach_error
                                }
                                Some(_) => ReceiverAttachError::NonAttachFrameReceived,
                                None => ReceiverAttachError::IllegalSessionState,
                            },
                            Err(_) => ReceiverAttachError::IllegalSessionState,
                        }
                    }
                    Err(_) => attach_error,
                }
            }
        }
    }
}

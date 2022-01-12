use std::{marker::PhantomData, sync::Arc};

use bytes::{Bytes, BytesMut};
use fe2o3_amqp_types::{
    definitions::AmqpError,
    messaging::{Address, DeliveryState, Message, Target},
    performatives::Transfer,
};
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;

use crate::{
    control::SessionControl,
    endpoint::Link,
    link::error::{detach_error_expecting_frame, map_send_detach_error},
    session::{self, SessionHandle},
};

use super::{
    builder::{self, WithTarget, WithoutName, WithoutTarget},
    delivery::Delivery,
    error::DetachError,
    role,
    state::{LinkFlowState, LinkState},
    type_state::{Attached, Detached},
    Error, LinkFrame, LinkHandle,
};

macro_rules! or_assign {
    ($self:ident, $other:ident, $field:ident) => {
        match &$self.performative.$field {
            Some(value) => {
                if let Some(other_value) = $other.$field {
                    if *value != other_value {
                        return Err(Error::AmqpError {
                            condition: AmqpError::NotAllowed,
                            description: Some(format!("Inconsistent {:?} in multi-frame delivery", value))
                        })
                    }
                }
            },
            None => {
                $self.performative.$field = $other.$field;
            }
        }
    };

    ($self:ident, $other:ident, $($field:ident), *) => {
        $(or_assign!($self, $other, $field);)*
    }
}

#[derive(Debug)]
pub(crate) struct IncompleteTransfer {
    pub performative: Transfer,
    pub payload: BytesMut,
}

impl IncompleteTransfer {
    /// Like `|=` operator but works on the field level
    pub fn or_assign(&mut self, other: Transfer) -> Result<(), Error> {
        or_assign! {
            self, other,
            delivery_id,
            delivery_tag,
            message_format
        };

        // If not set on the first (or only) transfer for a (multi-transfer)
        // delivery, then the settled flag MUST be interpreted as being false. For
        // subsequent transfers in a multi-transfer delivery if the settled flag
        // is left unset then it MUST be interpreted as true if and only if the
        // value of the settled flag on any of the preceding transfers was true;
        // if no preceding transfer was sent with settled being true then the
        // value when unset MUST be taken as false.
        match &self.performative.settled {
            Some(value) => {
                if let Some(other_value) = other.settled {
                    if !value {
                        self.performative.settled = Some(other_value);
                    }
                }
            }
            None => self.performative.settled = other.settled,
        }

        if let Some(other_state) = other.state {
            if let Some(state) = &self.performative.state {
                // Note that if the transfer performative (or an earlier disposition
                // performative referring to the delivery) indicates that the delivery has
                // attained a terminal state, then no future transfer or disposition sent
                // by the sender can alter that terminal state.
                if !state.is_terminal() {
                    self.performative.state = Some(other_state);
                }
            } else {
                self.performative.state = Some(other_state);
            }
        }

        Ok(())
    }

    /// Append to the buffered payload
    pub fn append(&mut self, other: Bytes) {
        self.payload.extend(other);
    }
}

type ReceiverFlowState = LinkFlowState<role::Receiver>;
type ReceiverLink = super::Link<role::Receiver, Arc<ReceiverFlowState>, DeliveryState>;

#[derive(Debug)]
pub struct Receiver<S> {
    pub(crate) link: ReceiverLink,
    pub(crate) buffer_size: usize,

    // Control sender to the session
    pub(crate) session: mpsc::Sender<SessionControl>,

    // Outgoing mpsc channel to send the Link Frames
    pub(crate) outgoing: PollSender<LinkFrame>,
    pub(crate) incoming: ReceiverStream<LinkFrame>,

    pub(crate) marker: PhantomData<S>,

    pub(crate) incomplete_transfer: Option<IncompleteTransfer>,
}

impl Receiver<Detached> {
    pub fn builder() -> builder::Builder<role::Receiver, WithoutName, WithTarget> {
        builder::Builder::new().target(Target::builder().build())
    }

    pub async fn attach(
        session: &mut SessionHandle,
        name: impl Into<String>,
        addr: impl Into<Address>,
    ) -> Result<Receiver<Attached>, Error> {
        Self::builder()
            .name(name)
            .source(addr)
            .attach(session)
            .await
    }

    async fn reattach_inner(
        mut self,
        mut session_control: mpsc::Sender<SessionControl>,
    ) -> Result<Receiver<Attached>, Error> {
        if self.link.output_handle.is_none() {
            let (tx, incoming) = mpsc::channel(self.buffer_size);
            let link_handle = LinkHandle::Receiver {
                tx,
                flow_state: self.link.flow_state.clone(),
                unsettled: self.link.unsettled.clone(),
                receiver_settle_mode: self.link.rcv_settle_mode.clone(),
                // This only controls whether a multi-transfer delivery id
                // will be added to sessions map
                more: false, 
            };
            self.incoming = ReceiverStream::new(incoming);
            let handle =
                session::allocate_link(&mut session_control, self.link.name.clone(), link_handle)
                    .await?;
            self.link.output_handle = Some(handle);
        }

        super::do_attach(&mut self.link, &mut self.outgoing, &mut self.incoming).await?;

        Ok(Receiver::<Attached> {
            link: self.link,
            buffer_size: self.buffer_size,
            session: self.session,
            outgoing: self.outgoing,
            incoming: self.incoming,
            marker: PhantomData,
            incomplete_transfer: None,
        })
    }
}

impl Receiver<Attached> {
    #[inline]
    async fn recv_inner(&mut self) -> Result<Option<Delivery>, Error> {
        let frame = self.incoming.next().await.ok_or_else(|| Error::AmqpError {
            condition: AmqpError::IllegalState,
            description: Some("Session is dropped".into()),
        })?;

        match frame {
            LinkFrame::Detach(detach) => {
                let err = DetachError {
                    link: Some(()),
                    is_closed_by_remote: detach.closed,
                    error: detach.error,
                };
                return Err(Error::Detached(err));
            }
            LinkFrame::Transfer {
                performative,
                payload,
            } => self.on_incoming_transfer(performative, payload).await,
            LinkFrame::Attach(_) => {
                return Err(Error::AmqpError {
                    condition: AmqpError::IllegalState,
                    description: Some("Received Attach on an attached link".into()),
                })
            }
            LinkFrame::Flow(_) | LinkFrame::Disposition(_) => {
                // Flow and Disposition are handled by LinkHandle which runs
                // in the session loop
                unreachable!()
            }
        }
    }

    pub async fn recv(&mut self) -> Result<Delivery, Error> {
        loop {
            match self.recv_inner().await? {
                Some(delivery) => return Ok(delivery),
                None => continue,
            }
        }
    }

    async fn on_incoming_transfer(
        &mut self,
        transfer: Transfer,
        payload: Bytes,
    ) -> Result<Option<Delivery>, Error> {
        use crate::endpoint::ReceiverLink;
        use futures_util::SinkExt;

        // Aborted messages SHOULD be discarded by the recipient (any payload
        // within the frame carrying the performative MUST be ignored). An aborted
        // message is implicitly settled
        if transfer.aborted {
            let _ = self.incomplete_transfer.take();
            return Ok(None);
        }

        let (delivery, disposition) = if transfer.more {
            // if
            todo!()
        } else {
            match self.incomplete_transfer.take() {
                Some(incomplete) => {
                    todo!()
                }
                None => self.link.on_incoming_transfer(transfer, payload).await?,
            }
        };

        // In `ReceiverSettleMode::First`, if the message is not pre-settled
        // the receiver will spontaneously settle the message with an
        // Accept by returning a `Some(Disposition)`
        if let Some(disposition) = disposition {
            let frame = LinkFrame::Disposition(disposition);
            self.outgoing.send(frame).await?;
        }

        Ok(Some(delivery))
    }

    // TODO: reduce mostly duplicated code (Sender/Receiver::detach/close)?
    pub async fn detach(self) -> Result<Receiver<Detached>, DetachError<Receiver<Detached>>> {
        println!(">>> Debug: Receiver::detach");
        let mut detaching = Receiver::<Detached> {
            link: self.link,
            buffer_size: self.buffer_size,
            session: self.session,
            outgoing: self.outgoing,
            incoming: self.incoming,
            marker: PhantomData,
            incomplete_transfer: self.incomplete_transfer,
        };

        // Send a non-closing detach
        match detaching
            .link
            .send_detach(&mut detaching.outgoing, false, None)
            .await
        {
            Ok(_) => {}
            Err(e) => return Err(map_send_detach_error(e, detaching)),
        };

        // Wait for remote detach
        let frame = match detaching.incoming.next().await {
            Some(frame) => frame,
            None => return Err(detach_error_expecting_frame(detaching)),
        };

        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame(detaching)),
        };

        if remote_detach.closed {
            // Note that one peer MAY send a closing detach while its partner is
            // sending a non-closing detach. In this case, the partner MUST
            // signal that it has closed the link by reattaching and then sending
            // a closing detach.
            let session_control = detaching.session.clone();
            let reattached = match detaching.reattach_inner(session_control).await {
                Ok(receiver) => receiver,
                Err(e) => {
                    //
                    println!("{:?}", e);
                    todo!()
                }
            };

            reattached.close().await?;

            // A peer closes a link by sending the detach frame with the handle for the
            // specified link, and the closed flag set to true. The partner will destroy
            // the corresponding link endpoint, and reply with its own detach frame with
            // the closed flag set to true.
            return Err(DetachError {
                link: None,
                is_closed_by_remote: true,
                error: None,
            });
        } else {
            match detaching.link.on_incoming_detach(remote_detach).await {
                Ok(_) => {}
                Err(e) => {
                    return Err(DetachError {
                        link: Some(detaching),
                        is_closed_by_remote: false,
                        error: Some(e),
                    })
                }
            }
        }

        match detaching
            .session
            .send(SessionControl::DeallocateLink(detaching.link.name.clone()))
            .await
        {
            Ok(_) => Ok(detaching),
            Err(e) => return Err(map_send_detach_error(e, detaching)),
        }
    }

    pub async fn close(self) -> Result<(), DetachError<Receiver<Detached>>> {
        let mut detaching = Receiver::<Detached> {
            link: self.link,
            buffer_size: self.buffer_size,
            session: self.session,
            outgoing: self.outgoing,
            incoming: self.incoming,
            marker: PhantomData,
            incomplete_transfer: self.incomplete_transfer,
        };

        // Send detach with closed=true and wait for remote closing detach
        // The sender will be dropped after close
        match detaching
            .link
            .send_detach(&mut detaching.outgoing, true, None)
            .await
        {
            Ok(_) => {}
            Err(e) => return Err(map_send_detach_error(e, detaching)),
        }

        // Wait for remote detach
        let frame = match detaching.incoming.next().await {
            Some(frame) => frame,
            None => return Err(detach_error_expecting_frame(detaching)),
        };
        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame(detaching)),
        };

        if remote_detach.closed {
            // If the remote detach contains an error, the error will be propagated
            // back by `on_incoming_detach`
            match detaching.link.on_incoming_detach(remote_detach).await {
                Ok(_) => {}
                Err(e) => {
                    return Err(DetachError {
                        link: Some(detaching),
                        is_closed_by_remote: false,
                        error: Some(e),
                    })
                }
            }
        } else {
            // Note that one peer MAY send a closing detach while its partner is
            // sending a non-closing detach. In this case, the partner MUST
            // signal that it has closed the link by reattaching and then sending
            // a closing detach.

            // Probably something like below
            // 1. wait for incoming attach
            // 2. send back attach
            // 3. wait for incoming closing detach
            // 4. detach

            todo!()
        }

        // TODO: de-allocate link from session
        match detaching.session
            .send(SessionControl::DeallocateLink(detaching.link.name.clone()))
            .await
            // .map_err(|e| map_send_detach_error)?;
        {
            Ok(_) => {},
            Err(e) => return Err(map_send_detach_error(e, detaching))
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use fe2o3_amqp_types::performatives::Transfer;

    use super::IncompleteTransfer;

    #[test]
    fn size_of_incomplete_transfer() {
        let size = std::mem::size_of::<Transfer>();
        println!("Transfer {:?}", size);

        let size = std::mem::size_of::<Option<IncompleteTransfer>>();
        println!("Option<IncompleteTransfer> {:?}", size);
    }
}
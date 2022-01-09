use std::{marker::PhantomData, sync::Arc};

use fe2o3_amqp_types::messaging::{Message, Target, Address};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;
use futures_util::StreamExt;

use crate::{session::{SessionHandle, self}, control::SessionControl, endpoint::Link, link::error::{map_send_detach_error, detach_error_expecting_frame}};

use super::{
    builder::{self, WithoutName, WithoutTarget, WithTarget},
    role, Error, LinkFrame, state::{LinkState, LinkFlowState}, type_state::{Detached, Attached}, error::DetachError, LinkHandle,
};

#[derive(Debug)]
pub struct Receiver<S> {
    pub(crate) link: super::Link<role::Receiver, Arc<LinkFlowState>>,
    pub(crate) buffer_size: usize,

    // Control sender to the session
    pub(crate) session: mpsc::Sender<SessionControl>,
    
    // Outgoing mpsc channel to send the Link Frames
    pub(crate) outgoing: PollSender<LinkFrame>,
    pub(crate) incoming: ReceiverStream<LinkFrame>,

    pub(crate) marker: PhantomData<S>,
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
                receiver_settle_mode: self.link.rcv_settle_mode.clone()
            };
            self.incoming = ReceiverStream::new(incoming);
            let handle = session::allocate_link(&mut session_control, self.link.name.clone(), link_handle).await?;
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
        })
    }
}

impl Receiver<Attached> {
    // TODO: reduce mostly duplicated code (Sender/Receiver::detach/close)?
    pub async fn detach(self) -> Result<Receiver<Detached>, DetachError<Receiver<Detached>>> {
        println!(">>> Debug: Receiver::detach");
        let mut detaching = Receiver::<Detached> {
            link: self.link,
            buffer_size: self.buffer_size,
            session: self.session,
            outgoing: self.outgoing,
            incoming: self.incoming,
            marker: PhantomData
        };

        // Send a non-closing detach
        match detaching.link.send_detach(&mut detaching.outgoing, false, None).await {
            Ok(_) => {},
            Err(e) => return Err(map_send_detach_error(e, detaching))
        };

        // Wait for remote detach
        let frame = match detaching.incoming.next().await {
            Some(frame) => frame,
            None => return Err(detach_error_expecting_frame(detaching))
        };

        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame(detaching))
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
                Ok(_) => {},
                Err(e) => return Err(DetachError {
                    link: Some(detaching),
                    is_closed_by_remote: false,
                    error: Some(e)
                })
            }
        }

        match detaching.session.send(SessionControl::DeallocateLink(detaching.link.name.clone()))
            .await {
            Ok(_) => Ok(detaching),
            Err(e) => return Err(map_send_detach_error(e, detaching))
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
        };

        // Send detach with closed=true and wait for remote closing detach
        // The sender will be dropped after close
        match detaching.link
            .send_detach(&mut detaching.outgoing, true, None)
            .await
        {
            Ok(_) => {},
            Err(e) => return Err(map_send_detach_error(e, detaching))
        }

        
        // Wait for remote detach
        let frame = match detaching
            .incoming
            .next()
            .await {
                Some(frame) => frame,
                None => return Err(detach_error_expecting_frame(detaching))
            };
        let remote_detach = match frame {
            LinkFrame::Detach(detach) => detach,
            _ => return Err(detach_error_expecting_frame(detaching)),
        };

        if remote_detach.closed {
            // If the remote detach contains an error, the error will be propagated
            // back by `on_incoming_detach`
            match detaching.link.on_incoming_detach(remote_detach).await {
                Ok(_) => {},
                Err(e) => return Err(DetachError {
                    link: Some(detaching),
                    is_closed_by_remote: false,
                    error: Some(e)
                })
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


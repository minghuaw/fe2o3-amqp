use fe2o3_amqp_types::{
    definitions::{self, AmqpError, SessionError},
    performatives::End,
};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

use crate::{
    connection::{self},
    control::{ConnectionControl, SessionControl},
    endpoint::{self, IncomingChannel, Session},
    link::LinkFrame,
    util::Running,
    SendBound,
};

use super::{
    error::{AllocLinkError, BeginError, Error, SessionInnerError},
    frame::{SessionIncomingItem, SessionOutgoingItem},
    SessionFrame, SessionFrameBody, SessionState,
};

async fn send_outgoing_item(
    outgoing: &mpsc::Sender<SessionFrame>,
    outgoing_item: SessionOutgoingItem,
) -> Result<(), SessionInnerError> {
    match outgoing_item {
        SessionOutgoingItem::SingleFrame(frame) => {
            outgoing
                .send(frame)
                .await
                // The receiving half must have dropped, and thus the `Connection`
                // event loop has stopped. It should be treated as an io error
                .map_err(|_| SessionInnerError::IllegalConnectionState)?;
        }
        SessionOutgoingItem::MultipleFrames(frames) => {
            for frame in frames {
                outgoing
                    .send(frame)
                    .await
                    // The receiving half must have dropped, and thus the `Connection`
                    // event loop has stopped. It should be treated as an io error
                    .map_err(|_| SessionInnerError::IllegalConnectionState)?;
            }
        }
    }
    Ok(())
}

pub(crate) struct SessionEngine<S: Session> {
    pub conn_control: mpsc::Sender<ConnectionControl>,
    pub session: S,
    pub control: mpsc::Receiver<SessionControl>,
    pub incoming: mpsc::Receiver<SessionIncomingItem>,
    pub outgoing: mpsc::Sender<SessionFrame>,

    pub outgoing_link_frames: mpsc::Receiver<LinkFrame>,
}

impl<S> SessionEngine<S>
where
    S: endpoint::Session,
    BeginError: From<S::BeginError>,
{
    pub(crate) async fn begin_client_session(
        conn_control: mpsc::Sender<ConnectionControl>,
        session: S,
        control: mpsc::Receiver<SessionControl>,
        incoming: mpsc::Receiver<SessionIncomingItem>,
        outgoing: mpsc::Sender<SessionFrame>,
        outgoing_link_frames: mpsc::Receiver<LinkFrame>,
    ) -> Result<Self, BeginError> {
        let mut engine = Self {
            conn_control,
            session,
            control,
            incoming,
            outgoing,
            outgoing_link_frames,
        };

        // send a begin
        engine.session.send_begin(&engine.outgoing).await?;
        // wait for an incoming begin
        let frame = match engine.incoming.recv().await {
            Some(frame) => frame,
            None => {
                // Connection sender must have dropped
                return Err(BeginError::IllegalConnectionState);
            }
        };
        let SessionFrame { channel, body } = frame;
        let channel = IncomingChannel(channel);
        let remote_begin = match body {
            SessionFrameBody::Begin(begin) => begin,
            SessionFrameBody::End(end) => match end.error {
                Some(err) => return Err(BeginError::RemoteEndedWithError(err)),
                None => return Err(BeginError::RemoteEnded),
            },
            _ => return Err(BeginError::IllegalState),
        };
        engine.session.on_incoming_begin(channel, remote_begin)?;
        Ok(engine)
    }
}

cfg_not_wasm32! {
    impl<S> SessionEngine<S>
    where
        S: endpoint::SessionEndpoint<State = SessionState> + Send + Sync + 'static,
        AllocLinkError: From<S::AllocError>,
        SessionInnerError: From<S::Error> + From<S::BeginError> + From<S::EndError>,
    {
        pub fn spawn(self) -> (JoinHandle<()>, oneshot::Receiver<Result<(), Error>>) {
            let (tx, rx) = oneshot::channel();
            let handle = tokio::spawn(self.event_loop(tx));
            (handle, rx)
        }
    }
}

cfg_wasm32! {
    impl<S> SessionEngine<S>
    where
        S: endpoint::SessionEndpoint<State = SessionState> + SendBound + Sync + 'static,
        AllocLinkError: From<S::AllocError>,
        SessionInnerError: From<S::Error> + From<S::BeginError> + From<S::EndError>,
    {
        pub fn spawn_local(self) -> (JoinHandle<()>, oneshot::Receiver<Result<(), Error>>) {
            let (tx, rx) = oneshot::channel();
            let handle = tokio::task::spawn_local(self.event_loop(tx));
            (handle, rx)
        }

        pub fn spawn_on_local_set(self, local_set: &tokio::task::LocalSet) -> (JoinHandle<()>, oneshot::Receiver<Result<(), Error>>) {
            let (tx, rx) = oneshot::channel();
            let handle = local_set.spawn_local(self.event_loop(tx));
            (handle, rx)
        }
    }
}

impl<S> SessionEngine<S>
where
    S: endpoint::SessionEndpoint<State = SessionState> + SendBound + Sync + 'static,
    AllocLinkError: From<S::AllocError>,
    SessionInnerError: From<S::Error> + From<S::BeginError> + From<S::EndError>,
{
    #[inline]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn on_incoming(
        &mut self,
        incoming: SessionIncomingItem,
    ) -> Result<Running, SessionInnerError> {
        let SessionFrame { channel, body } = incoming;
        let channel = IncomingChannel(channel);
        match body {
            SessionFrameBody::Begin(begin) => {
                self.session.on_incoming_begin(channel, begin)?;
            }
            SessionFrameBody::Attach(attach) => {
                self.session.on_incoming_attach(attach).await?;
            }
            SessionFrameBody::Flow(flow) => {
                if let Some(outgoing_item) = self.session.on_incoming_flow(flow).await? {
                    send_outgoing_item(&self.outgoing, outgoing_item).await?;
                }
            }
            SessionFrameBody::Transfer {
                performative,
                payload,
            } => {
                self.session
                    .on_incoming_transfer(performative, payload)
                    .await?;
            }
            SessionFrameBody::Disposition(disposition) => {
                if let Some(dispositions) = self.session.on_incoming_disposition(disposition)? {
                    for disposition in dispositions {
                        let disposition = self.session.on_outgoing_disposition(disposition)?;
                        self.outgoing
                            .send(disposition)
                            .await
                            // The receiving half must have dropped, and thus the `Connection`
                            // event loop has stopped. It should be treated as an io error
                            .map_err(|_| SessionInnerError::IllegalConnectionState)?;
                    }
                }
            }
            SessionFrameBody::Detach(detach) => {
                self.session.on_incoming_detach(detach).await?;
            }
            SessionFrameBody::End(end) => {
                let result = self.session.on_incoming_end(channel, end);
                if matches!(self.session.local_state(), SessionState::EndReceived) {
                    // if control is closing, finish sending all buffered messages before closing
                    self.outgoing_link_frames.close();
                    while let Some(frame) = self.outgoing_link_frames.recv().await {
                        self.on_outgoing_link_frames(frame).await?;
                    }

                    self.session.send_end(&self.outgoing, None).await?;
                }
                result?;
            }
        }

        match self.session.local_state() {
            SessionState::Unmapped => Ok(Running::Stop),
            _ => Ok(Running::Continue),
        }
    }

    #[inline]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn on_control(&mut self, control: SessionControl) -> Result<Running, SessionInnerError> {
        #[cfg(feature = "tracing")]
        tracing::trace!("control: {}", control);
        #[cfg(feature = "log")]
        log::trace!("control: {}", control);
        match control {
            SessionControl::End(error) => {
                // if control is closing, finish sending all buffered messages before closing
                self.outgoing_link_frames.close();
                while let Some(frame) = self.outgoing_link_frames.recv().await {
                    self.on_outgoing_link_frames(frame).await?;
                }

                self.session.send_end(&self.outgoing, error).await?;
            }
            SessionControl::AllocateLink {
                link_name,
                link_relay,
                responder,
            } => {
                let result = self.session.allocate_link(link_name, Some(link_relay));
                responder
                    .send(result.map_err(Into::into))
                    // The receiving end (ie. link) must have been stopped
                    .map_err(|_| SessionInnerError::UnattachedHandle)?;
            }
            SessionControl::AllocateIncomingLink {
                link_name,
                link_relay,
                input_handle,
                responder,
            } => {
                let result =
                    self.session
                        .allocate_incoming_link(link_name, link_relay, input_handle);
                responder
                    .send(result.map_err(Into::into))
                    // The receiving end (ie. link) must have been stopped
                    .map_err(|_| SessionInnerError::UnattachedHandle)?;
            }
            SessionControl::DeallocateLink(link_name) => {
                self.session.deallocate_link(link_name);
            }
            SessionControl::Disposition(disposition) => {
                let disposition = self.session.on_outgoing_disposition(disposition)?;
                self.outgoing
                    .send(disposition)
                    .await
                    // The receiving half must have dropped, and thus the `Connection`
                    // event loop has stopped. It should be treated as an io error
                    .map_err(|_| SessionInnerError::IllegalConnectionState)?;
            }
            SessionControl::CloseConnectionWithError((condition, description)) => {
                let error = definitions::Error::new(condition, description, None);
                let control = ConnectionControl::Close(Some(error));
                self.conn_control
                    .send(control)
                    .await
                    .map_err(|_| SessionInnerError::IllegalConnectionState)?;
            }
            SessionControl::GetMaxFrameSize(resp) => {
                self.conn_control
                    .send(ConnectionControl::GetMaxFrameSize(resp))
                    .await
                    .map_err(|_| SessionInnerError::IllegalConnectionState)?;
            }

            #[cfg(feature = "transaction")]
            SessionControl::AllocateTransactionId { resp } => {
                let result = self.session.allocate_transaction_id();
                resp.send(result)
                    .map_err(|_| SessionInnerError::UnattachedHandle)?;
            }
            #[cfg(feature = "transaction")]
            SessionControl::CommitTransaction { txn_id, resp } => {
                let result = self.session.commit_transaction(txn_id).await?;
                resp.send(result)
                    .map_err(|_| SessionInnerError::UnattachedHandle)?;
            }
            #[cfg(feature = "transaction")]
            SessionControl::RollbackTransaction { txn_id, resp } => {
                let result = self.session.rollback_transaction(txn_id)?;
                resp.send(result)
                    .map_err(|_| SessionInnerError::UnattachedHandle)?;
            }
            #[cfg(feature = "transaction")]
            SessionControl::AbortTransaction(txn_id) => {
                let _ = self.session.rollback_transaction(txn_id);
            }
        }

        match self.session.local_state() {
            SessionState::Unmapped => Ok(Running::Stop),
            _ => Ok(Running::Continue),
        }
    }

    #[inline]
    async fn on_outgoing_link_frames(
        &mut self,
        frame: LinkFrame,
    ) -> Result<Running, SessionInnerError> {
        match self.session.local_state() {
            SessionState::Mapped => {}
            _ => return Err(SessionInnerError::IllegalState), // End session with illegal state
        }

        let outgoing_item = match frame {
            LinkFrame::Attach(attach) => self
                .session
                .on_outgoing_attach(attach)
                .map(SessionOutgoingItem::SingleFrame)
                .map(Some)?,
            LinkFrame::Flow(flow) => self
                .session
                .on_outgoing_flow(flow)
                .map(SessionOutgoingItem::SingleFrame)
                .map(Some)?,
            LinkFrame::Transfer {
                input_handle,
                performative,
                payload,
            } => self
                .session
                .on_outgoing_transfer(input_handle, performative, payload)?,
            LinkFrame::Disposition(disposition) => self
                .session
                .on_outgoing_disposition(disposition)
                .map(SessionOutgoingItem::SingleFrame)
                .map(Some)?,
            LinkFrame::Detach(detach) => Some(SessionOutgoingItem::SingleFrame(
                self.session.on_outgoing_detach(detach),
            )),

            #[cfg(feature = "transaction")]
            LinkFrame::Acquisition(_) => {
                // This is purely used to notify sender about TxnAcquisition, which is not implemented
                unreachable!("LinkFrame::Acquisition should not appear in outgoing link frames")
            }
        };

        if let Some(outgoing_item) = outgoing_item {
            send_outgoing_item(&self.outgoing, outgoing_item).await?;
        }

        match self.session.local_state() {
            SessionState::Unmapped => Ok(Running::Stop),
            _ => Ok(Running::Continue),
        }
    }

    #[inline]
    async fn on_error(&mut self, kind: &SessionInnerError) -> Result<Running, SessionInnerError> {
        use definitions::Error;

        #[cfg(not(target_arch = "wasm32"))]
        #[cfg(all(feature = "transaction", feature = "acceptor"))]
        use fe2o3_amqp_types::transaction::TransactionError;

        match kind {
            SessionInnerError::UnattachedHandle => {
                let error = Error::new(SessionError::UnattachedHandle, None, None);
                self.end_session(Some(error)).await
            }
            SessionInnerError::RemoteAttachingLinkNameNotFound => {
                let error = Error::new(
                    AmqpError::InternalError,
                    Some(String::from("Link name is not found")),
                    None,
                );
                self.end_session(Some(error)).await
            }
            SessionInnerError::HandleInUse => {
                let error = Error::new(SessionError::HandleInUse, None, None);
                self.end_session(Some(error)).await
            }
            SessionInnerError::IllegalState => {
                let error = Error::new(AmqpError::IllegalState, None, None);
                self.end_session(Some(error)).await
            }
            SessionInnerError::IllegalConnectionState => Ok(Running::Stop),
            SessionInnerError::TransferFrameToSender => {
                let error = Error::new(
                    AmqpError::NotAllowed,
                    Some(String::from("Found Transfer frame sent Sender link")),
                    None,
                );
                self.end_session(Some(error)).await
            }
            SessionInnerError::RemoteEnded | SessionInnerError::RemoteEndedWithError(_) => {
                self.end_session(None).await
            }

            #[cfg(not(target_arch = "wasm32"))]
            #[cfg(all(feature = "transaction", feature = "acceptor"))]
            SessionInnerError::UnknownTxnId => {
                let error = Error::new(TransactionError::UnknownId, None, None);
                self.end_session(Some(error)).await
            }
        }
    }

    async fn end_session(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<Running, SessionInnerError> {
        match self.session.local_state() {
            SessionState::Unmapped => {}
            SessionState::BeginSent | SessionState::BeginReceived | SessionState::Mapped => {
                self.session
                    .send_end(&self.outgoing, error)
                    .await
                    .map_err(|_| SessionInnerError::IllegalConnectionState)?;
                let (channel, end) = self.wait_for_remote_end(false).await?;
                self.session.on_incoming_end(channel, end)?;
            }
            SessionState::EndSent => {
                self.wait_for_remote_end(false).await?;
            }
            SessionState::EndReceived => {
                self.session
                    .send_end(&self.outgoing, error)
                    .await
                    .map_err(|_| SessionInnerError::IllegalConnectionState)?;
            }
            SessionState::Discarding => {
                // The DISCARDING state is a variant of the CLOSE SENT state where the close is triggered
                // by an error. In this case any incoming frames on the connection MUST be silently discarded
                // until the peer’s close frame is received.
                self.wait_for_remote_end(true).await?;
            }
        }

        Ok(Running::Stop)
    }

    async fn wait_for_remote_end(
        &mut self,
        discard_other_frame: bool,
    ) -> Result<(IncomingChannel, End), SessionInnerError> {
        loop {
            let frame = self
                .incoming
                .recv()
                .await
                .ok_or(SessionInnerError::IllegalConnectionState)?;
            match frame.body {
                SessionFrameBody::End(end) => return Ok((IncomingChannel(frame.channel), end)),
                _ => {
                    if !discard_other_frame {
                        let _ = self.on_incoming(frame).await?;
                    }
                }
            }
        }
    }

    #[inline]
    fn continue_or_stop_by_state(&self) -> Running {
        match self.session.local_state() {
            SessionState::BeginSent
            | SessionState::BeginReceived
            | SessionState::Mapped
            | SessionState::EndSent
            | SessionState::EndReceived => Running::Continue,
            SessionState::Unmapped | SessionState::Discarding => Running::Stop,
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(name = "Session::event_loop", skip(self), fields(outgoing_channel = %self.session.outgoing_channel().0)))]
    async fn event_loop(mut self, tx: oneshot::Sender<Result<(), Error>>) {
        let mut outcome = Ok(());
        loop {
            let result = tokio::select! {
                incoming = self.incoming.recv() => {
                    match incoming {
                        Some(incoming) => self.on_incoming(incoming).await,
                        None => {
                            // Check local state
                            match self.continue_or_stop_by_state() {
                                Running::Continue => {
                                    // The connection must have already stopped before session negotiated ending
                                    Err(SessionInnerError::IllegalConnectionState)
                                },
                                Running::Stop => Ok(Running::Stop),
                            }
                        }
                    }
                },
                control = self.control.recv() => {
                    match control {
                        Some(control) => {
                            self.on_control(control).await
                        },
                        None => {
                            // all Links and SessionHandle are dropped
                            // Complete ending session
                            'inner: loop {
                                match self.session.local_state() {
                                    SessionState::Mapped
                                    | SessionState::EndReceived => {
                                        if let Err(_err) = self.on_control(SessionControl::End(None)).await {
                                            #[cfg(feature = "tracing")]
                                            tracing::error!("err {:?}", _err);
                                            #[cfg(feature = "log")]
                                            log::error!("err {:?}", _err);
                                        }
                                    },
                                    // Wait for remote end
                                    SessionState::Discarding | SessionState::EndSent => {
                                        match self.incoming.recv().await {
                                            Some(incoming) => {
                                                let _ = self.on_incoming(incoming).await;
                                            },
                                            None => {
                                                // The connection must have already stopped before session negotiated ending
                                                break 'inner
                                            }
                                        }
                                    },
                                    SessionState::Unmapped => break 'inner,
                                    _ => break 'inner,
                                }
                            }

                            Ok(Running::Stop)
                        }
                    }
                },
                frame = self.outgoing_link_frames.recv() => {
                    match frame {
                        Some(frame) => self.on_outgoing_link_frames(frame).await,
                        None => {
                            // All Links and SessionHandle are dropped
                            //
                            // Upon ending, all link-to-session channels will be closed
                            // first while the session is still waitint for remote end frame.
                            Ok(Running::Continue)
                        }
                    }
                }
            };

            let running = match result {
                Ok(running) => running,
                Err(error) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!("{:?}", error);
                    #[cfg(feature = "log")]
                    log::error!("{:?}", error);
                    match self.on_error(&error).await {
                        Ok(running) => {
                            outcome = Err(error);
                            running
                        }
                        Err(error) => {
                            // Stop the session if error cannot be handled
                            outcome = Err(error);
                            Running::Stop
                        }
                    }
                }
            };

            match running {
                Running::Continue => {}
                Running::Stop => break,
            }
        }

        #[cfg(feature = "tracing")]
        tracing::debug!("Stopped");
        #[cfg(feature = "log")]
        log::debug!("Stopped");
        let _ =
            connection::deallocate_session(&mut self.conn_control, self.session.outgoing_channel())
                .await;
        let result = outcome.map_err(Into::into);
        let _ = tx.send(result);
    }
}

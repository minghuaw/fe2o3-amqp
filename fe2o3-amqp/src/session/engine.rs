use fe2o3_amqp_types::{
    definitions::{self, AmqpError, SessionError},
    performatives::End,
};
use tokio::{sync::mpsc, task::JoinHandle};

use tracing::{debug, error, instrument, trace};

use crate::{
    connection::{self},
    control::{ConnectionControl, SessionControl},
    endpoint::{self, IncomingChannel, Session},
    link::LinkFrame,
    util::Running,
};

use super::{
    error::{AllocLinkError, SessionBeginError, SessionErrorKind, SessionInnerError},
    frame::SessionIncomingItem,
    SessionFrame, SessionFrameBody, SessionState,
};

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
    SessionBeginError: From<S::BeginError>,
{
    pub(crate) async fn begin_client_session(
        conn_control: mpsc::Sender<ConnectionControl>,
        session: S,
        control: mpsc::Receiver<SessionControl>,
        incoming: mpsc::Receiver<SessionIncomingItem>,
        outgoing: mpsc::Sender<SessionFrame>,
        outgoing_link_frames: mpsc::Receiver<LinkFrame>,
    ) -> Result<Self, SessionBeginError> {
        let mut engine = Self {
            conn_control,
            session,
            control,
            incoming,
            outgoing,
            outgoing_link_frames,
        };

        // send a begin
        engine.session.send_begin(&mut engine.outgoing).await?;
        // wait for an incoming begin
        let frame = match engine.incoming.recv().await {
            Some(frame) => frame,
            None => {
                // Connection sender must have dropped
                return Err(SessionBeginError::IllegalConnectionState);
            }
        };
        let SessionFrame { channel, body } = frame;
        let channel = IncomingChannel(channel);
        let remote_begin = match body {
            SessionFrameBody::Begin(begin) => begin,
            SessionFrameBody::End(end) => match end.error {
                Some(err) => return Err(SessionBeginError::RemoteEndedWithError(err)),
                None => return Err(SessionBeginError::RemoteEnded),
            },
            _ => return Err(SessionBeginError::IllegalState),
        };
        engine.session.on_incoming_begin(channel, remote_begin)?;
        Ok(engine)
    }
}

impl<S> SessionEngine<S>
where
    // S: endpoint::Session<State = SessionState> + Send + Sync + 'static,
    S: endpoint::SessionEndpoint<State = SessionState> + Send + Sync + 'static,
    AllocLinkError: From<S::AllocError>,
    SessionInnerError: From<S::Error> + From<S::BeginError> + From<S::EndError>,
{
    pub fn spawn(self) -> JoinHandle<Result<(), SessionErrorKind>> {
        tokio::spawn(self.event_loop())
    }

    #[inline]
    #[instrument(skip_all)]
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
                self.session.on_incoming_flow(flow).await?;
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
                self.session.on_incoming_disposition(disposition).await?;
            }
            SessionFrameBody::Detach(detach) => {
                self.session.on_incoming_detach(detach).await?;
            }
            SessionFrameBody::End(end) => {
                self.session.on_incoming_end(channel, end).await?;
            }
        }

        match self.session.local_state() {
            SessionState::Unmapped => Ok(Running::Stop),
            _ => Ok(Running::Continue),
        }
    }

    #[inline]
    #[instrument(skip_all)]
    async fn on_control(&mut self, control: SessionControl) -> Result<Running, SessionInnerError> {
        trace!("control: {}", control);
        match control {
            SessionControl::End(error) => {
                // if control is closing, finish sending all buffered messages before closing
                self.outgoing_link_frames.close();
                while let Some(frame) = self.outgoing_link_frames.recv().await {
                    self.on_outgoing_link_frames(frame).await?;
                }

                self.session.send_end(&mut self.outgoing, error).await?;
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
            SessionControl::LinkFlow(flow) => {
                let flow = self.session.on_outgoing_flow(flow)?;
                self.outgoing
                    .send(flow)
                    .await
                    // The receiving half must have dropped, and thus the `Connection`
                    // event loop has stopped. It should be treated as an io error
                    .map_err(|_| SessionInnerError::IllegalConnectionState)?;
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

        let session_frame = match frame {
            LinkFrame::Attach(attach) => self.session.on_outgoing_attach(attach)?,
            LinkFrame::Flow(flow) => self.session.on_outgoing_flow(flow)?,
            LinkFrame::Transfer {
                input_handle,
                performative,
                payload,
            } => self
                .session
                .on_outgoing_transfer(input_handle, performative, payload)?,
            LinkFrame::Disposition(disposition) => {
                self.session.on_outgoing_disposition(disposition)?
            }
            LinkFrame::Detach(detach) => self.session.on_outgoing_detach(detach),

            #[cfg(feature = "transaction")]
            LinkFrame::Acquisition(_) => {
                // This is purely used to notify sender about TxnAcquisition, which is not implemented
                unreachable!("LinkFrame::Acquisition should not appear in outgoing link frames")
            }
        };

        self.outgoing
            .send(session_frame)
            .await
            // The receiving half must have dropped, and thus the `Connection`
            // event loop has stopped. It should be treated as an io error
            .map_err(|_| SessionInnerError::IllegalConnectionState)?;

        match self.session.local_state() {
            SessionState::Unmapped => Ok(Running::Stop),
            _ => Ok(Running::Continue),
        }
    }

    #[inline]
    async fn on_error(&mut self, kind: &SessionInnerError) -> Result<Running, SessionInnerError> {
        use definitions::Error;

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
            },
            SessionInnerError::IllegalState => {
                let error = Error::new(AmqpError::IllegalState, None, None);
                self.end_session(Some(error)).await
            },
            SessionInnerError::IllegalConnectionState => Ok(Running::Stop),
            SessionInnerError::TransferFrameToSender => {
                let error = Error::new(
                    AmqpError::NotAllowed,
                    Some(String::from("Found Transfer frame sent Sender link")),
                    None,
                );
                self.end_session(Some(error)).await
            },
            SessionInnerError::RemoteEnded => self.end_session(None).await,
            SessionInnerError::RemoteEndedWithError(_) => self.end_session(None).await,
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
                self.session.on_incoming_end(channel, end).await?;
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

    // #[inline]
    // async fn on_link_handle_error(
    //     &mut self,
    //     handle: &Handle,
    //     closed: &bool,
    //     error: LinkRelayError,
    // ) -> Result<Running, SessionErrorKind> {
    //     // TODO: detach?
    //     let detach = Detach {
    //         handle: handle.clone(),
    //         closed: *closed,
    //         error: Some(definitions::Error::from(error)),
    //     };
    //     let frame = self.session.on_outgoing_detach(detach);
    //     if let Err(error) = self.outgoing.send(frame).await {
    //         // The connection must have dropped
    //         Err(SessionErrorKind::IllegalConnectionState)
    //     } else {
    //         // Overall, link errors should not stop the session
    //         Ok(Running::Continue)
    //     }
    // }

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

    #[instrument(name = "Session::event_loop", skip(self), fields(outgoing_channel = %self.session.outgoing_channel().0))]
    async fn event_loop(mut self) -> Result<(), SessionErrorKind> {
        let mut outcome = Ok(());
        loop {
            let result = tokio::select! {
                incoming = self.incoming.recv() => {
                    let result = match incoming {
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
                    };
                    result
                },
                control = self.control.recv() => {
                    let result = match control {
                        Some(control) => {
                            self.on_control(control).await
                        },
                        None => {
                            // all Links and SessionHandle are dropped
                            Ok(Running::Stop)
                        }
                    };
                    result
                },
                frame = self.outgoing_link_frames.recv() => {
                    let result = match frame {
                        Some(frame) => self.on_outgoing_link_frames(frame).await,
                        None => {
                            // All Links and SessionHandle are dropped
                            //
                            // Upon ending, all link-to-session channels will be closed
                            // first while the session is still waitint for remote end frame.
                            Ok(Running::Continue)
                        }
                    };
                    result
                }
            };

            let running = match result {
                Ok(running) => running,
                Err(error) => {
                    error!("{:?}", error);
                    match self.on_error(&error).await {
                        Ok(running) => {
                            outcome = Err(error);
                            running
                        }
                        Err(error) => {
                            // Stop the session if errors cannot be handled
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

        debug!("Stopped");
        let _ =
            connection::deallocate_session(&mut self.conn_control, self.session.outgoing_channel())
                .await;
        outcome.map_err(Into::into)
    }
}

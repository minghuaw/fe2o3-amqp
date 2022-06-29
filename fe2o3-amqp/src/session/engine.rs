use std::{fmt::Debug, io};

use fe2o3_amqp_types::{
    definitions::{self, AmqpError, ConnectionError, Handle},
    performatives::Detach,
};
use futures_util::SinkExt;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::PollSender;
use tracing::{debug, error, instrument, trace};

use crate::{
    connection::{self},
    control::{ConnectionControl, SessionControl},
    endpoint::{self, IncomingChannel, Session},
    link::LinkFrame,
    util::Running,
};

use super::{
    error::AllocLinkError, frame::SessionIncomingItem, Error, SessionFrame, SessionFrameBody,
    SessionState,
};

pub(crate) struct SessionEngine<S: Session> {
    pub conn: mpsc::Sender<ConnectionControl>,
    pub session: S,
    pub control: mpsc::Receiver<SessionControl>,
    pub incoming: mpsc::Receiver<SessionIncomingItem>,
    pub outgoing: PollSender<SessionFrame>,

    pub outgoing_link_frames: mpsc::Receiver<LinkFrame>,
}

impl<S> SessionEngine<S>
where
    S: endpoint::Session<Error = Error>,
{
    pub(crate) async fn begin_client_session(
        conn: mpsc::Sender<ConnectionControl>,
        session: S,
        control: mpsc::Receiver<SessionControl>,
        incoming: mpsc::Receiver<SessionIncomingItem>,
        outgoing: PollSender<SessionFrame>,
        outgoing_link_frames: mpsc::Receiver<LinkFrame>,
    ) -> Result<Self, Error> {
        let mut engine = Self {
            conn,
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
                return Err(Error::Io(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Connection sender must have dropped",
                )));
            }
        };
        let SessionFrame { channel, body } = frame;
        let channel = IncomingChannel(channel);
        let remote_begin = match body {
            SessionFrameBody::Begin(begin) => begin,
            SessionFrameBody::End(end) => {
                return Err(Error::Remote(end.error.unwrap_or_else(|| {
                    definitions::Error::new(
                        AmqpError::InternalError,
                        Some("Remote closed session wihtout explanation".into()),
                        None,
                    )
                })))
            }
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)), // End session with illegal state
        };
        engine.session.on_incoming_begin(channel, remote_begin)?;
        Ok(engine)
    }
}

impl<S> SessionEngine<S>
where
    // S: endpoint::Session<State = SessionState> + Send + Sync + 'static,
    S: endpoint::SessionEndpoint<State = SessionState> + Send + Sync + 'static,
    S::Error: Into<Error> + Debug,
    S::AllocError: Into<AllocLinkError>,
{
    pub fn spawn(self) -> JoinHandle<Result<(), Error>> {
        tokio::spawn(self.event_loop())
    }

    #[inline]
    #[instrument(skip_all)]
    async fn on_incoming(&mut self, incoming: SessionIncomingItem) -> Result<Running, Error> {
        let SessionFrame { channel, body } = incoming;
        let channel = IncomingChannel(channel);
        match body {
            SessionFrameBody::Begin(begin) => {
                self.session
                    .on_incoming_begin(channel, begin)
                    .map_err(Into::into)?;
            }
            SessionFrameBody::Attach(attach) => {
                self.session
                    .on_incoming_attach(attach)
                    .await
                    .map_err(Into::into)?;
            }
            SessionFrameBody::Flow(flow) => {
                self.session
                    .on_incoming_flow(flow)
                    .await
                    .map_err(Into::into)?;
            }
            SessionFrameBody::Transfer {
                performative,
                payload,
            } => {
                self.session
                    .on_incoming_transfer(performative, payload)
                    .await
                    .map_err(Into::into)?;
            }
            SessionFrameBody::Disposition(disposition) => {
                self.session
                    .on_incoming_disposition(disposition)
                    .await
                    .map_err(Into::into)?;
            }
            SessionFrameBody::Detach(detach) => {
                self.session
                    .on_incoming_detach(detach)
                    .await
                    .map_err(Into::into)?;
            }
            SessionFrameBody::End(end) => {
                self.session
                    .on_incoming_end(channel, end)
                    .await
                    .map_err(Into::into)?;
            }
        }

        match self.session.local_state() {
            SessionState::Unmapped => Ok(Running::Stop),
            _ => Ok(Running::Continue),
        }
    }

    #[inline]
    #[instrument(skip_all)]
    async fn on_control(&mut self, control: SessionControl) -> Result<Running, Error> {
        trace!("control: {}", control);
        match control {
            SessionControl::End(error) => {
                // if control is closing, finish sending all buffered messages before closing
                self.outgoing_link_frames.close();
                while let Some(frame) = self.outgoing_link_frames.recv().await {
                    self.on_outgoing_link_frames(frame).await?;
                }

                self.session
                    .send_end(&mut self.outgoing, error)
                    .await
                    .map_err(Into::into)?;
            }
            SessionControl::AllocateLink {
                link_name,
                link_relay,
                responder,
            } => {
                let result = self.session.allocate_link(link_name, Some(link_relay));
                responder.send(result.map_err(Into::into)).map_err(|_| {
                    Error::Io(io::Error::new(
                        io::ErrorKind::Other,
                        "SessionHandle is dropped",
                    ))
                })?;
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
                responder.send(result.map_err(Into::into)).map_err(|_| {
                    Error::Io(io::Error::new(
                        io::ErrorKind::Other,
                        "SessionHandle is dropped",
                    ))
                })?;
            }
            SessionControl::DeallocateLink(link_name) => {
                self.session.deallocate_link(link_name);
            }
            SessionControl::LinkFlow(flow) => {
                let flow = self.session.on_outgoing_flow(flow).map_err(Into::into)?;
                self.outgoing
                    .send(flow)
                    .await
                    // The receiving half must have dropped, and thus the `Connection`
                    // event loop has stopped. It should be treated as an io error
                    .map_err(|e| Error::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;
            }
            SessionControl::Disposition(disposition) => {
                let disposition = self
                    .session
                    .on_outgoing_disposition(disposition)
                    .map_err(Into::into)?;
                self.outgoing
                    .send(disposition)
                    .await
                    // The receiving half must have dropped, and thus the `Connection`
                    // event loop has stopped. It should be treated as an io error
                    .map_err(|e| Error::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;
            }
            SessionControl::CloseConnectionWithError((condition, description)) => {
                let error = definitions::Error::new(condition, description, None);
                let control = ConnectionControl::Close(Some(error));
                self.conn
                    .send(control)
                    .await
                    .map_err(|e| Error::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;
            }

            #[cfg(feature = "transaction")]
            SessionControl::AllocateTransactionId { resp } => {
                let result = self.session.allocate_transaction_id();
                resp.send(result).map_err(|_| {
                    Error::Io(io::Error::new(
                        io::ErrorKind::Other,
                        "Coorindator is dropped",
                    ))
                })?;
            }
            #[cfg(feature = "transaction")]
            SessionControl::CommitTransaction { txn_id, resp } => {
                let result = self
                    .session
                    .commit_transaction(txn_id)
                    .await
                    .map_err(Into::into)?;
                resp.send(result).map_err(|_| {
                    Error::Io(io::Error::new(
                        io::ErrorKind::Other,
                        "Coorindator is dropped",
                    ))
                })?;
            }
            #[cfg(feature = "transaction")]
            SessionControl::RollbackTransaction { txn_id, resp } => {
                let result = self
                    .session
                    .rollback_transaction(txn_id)
                    .await
                    .map_err(Into::into)?;
                resp.send(result).map_err(|_| {
                    Error::Io(io::Error::new(
                        io::ErrorKind::Other,
                        "Coorindator is dropped",
                    ))
                })?;
            }
            #[cfg(feature = "transaction")]
            SessionControl::AbortTransaction(txn_id) => {
                let _ = self.session.rollback_transaction(txn_id).await;
            }
        }

        match self.session.local_state() {
            SessionState::Unmapped => Ok(Running::Stop),
            _ => Ok(Running::Continue),
        }
    }

    #[inline]
    #[instrument(skip_all)]
    async fn on_outgoing_link_frames(&mut self, frame: LinkFrame) -> Result<Running, Error> {
        trace!(state = ?self.session.local_state(), frame = ?frame);
        match self.session.local_state() {
            SessionState::Mapped => {}
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)), // End session with illegal state
        }

        let session_frame = match frame {
            LinkFrame::Attach(attach) => self
                .session
                .on_outgoing_attach(attach)
                .map_err(Into::into)?,
            LinkFrame::Flow(flow) => self.session.on_outgoing_flow(flow).map_err(Into::into)?,
            LinkFrame::Transfer {
                input_handle,
                performative,
                payload,
            } => self
                .session
                .on_outgoing_transfer(input_handle, performative, payload)
                .map_err(Into::into)?,
            LinkFrame::Disposition(disposition) => self
                .session
                .on_outgoing_disposition(disposition)
                .map_err(Into::into)?,
            LinkFrame::Detach(detach) => self
                .session
                .on_outgoing_detach(detach)
                .map_err(Into::into)?,
            LinkFrame::Acquisition(_) => {
                return Err(Error::Local(definitions::Error::new(
                    AmqpError::InternalError,
                    "Acquisition is not expected in outgoing link frames".to_string(),
                    None,
                )))
            }
        };

        self.outgoing
            .send(session_frame)
            .await
            // The receiving half must have dropped, and thus the `Connection`
            // event loop has stopped. It should be treated as an io error
            .map_err(|e| Error::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;

        match self.session.local_state() {
            SessionState::Unmapped => Ok(Running::Stop),
            _ => Ok(Running::Continue),
        }
    }

    #[inline]
    async fn on_error(&mut self, error: &Error) -> Running {
        match error {
            Error::Io(_) => Running::Stop,
            Error::ChannelMaxReached => Running::Stop,
            Error::JoinError(_) => unreachable!(),
            Error::Local(error) => {
                let _ = self
                    .session
                    .send_end(&mut self.outgoing, Some(error.clone()))
                    .await;
                self.continue_or_stop_by_state()
            }
            Error::Remote(_) => self.continue_or_stop_by_state(),
            Error::LinkHandleError {
                handle,
                closed,
                error,
            } => self.on_link_handle_error(handle, closed, error).await,
            Error::HandleMaxExceeded => {
                let error = definitions::Error::new(
                    ConnectionError::FramingError,
                    "A handle outside the supported range is received".to_string(),
                    None,
                );
                let _ = self.conn.send(ConnectionControl::Close(Some(error))).await;
                Running::Stop
            }
            #[cfg(feature = "transaction")]
            Error::CoordinatorAttachError(error) => {
                // TODO: just log the error?
                tracing::error!(?error);
                Running::Continue
            }
        }
    }

    #[inline]
    async fn on_link_handle_error(
        &mut self,
        handle: &Handle,
        closed: &bool,
        error: &definitions::Error,
    ) -> Running {
        // TODO: detach?
        let detach = Detach {
            handle: handle.clone(),
            closed: *closed,
            error: Some(error.clone()),
        };
        match self.session.on_outgoing_detach(detach) {
            Ok(frame) => {
                if let Err(error) = self.outgoing.send(frame).await {
                    // The connection must have dropped
                    error!(?error);
                    return Running::Stop;
                }
            }
            Err(error) => error!(?error),
        }
        // Overall, link errors should not stop the session
        Running::Continue
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

    #[instrument(name = "Session::event_loop", skip(self), fields(outgoing_channel = %self.session.outgoing_channel().0))]
    async fn event_loop(mut self) -> Result<(), Error> {
        let mut outcome = Ok(());
        loop {
            let result = tokio::select! {
                incoming = self.incoming.recv() => {
                    let result = match incoming {
                        Some(incoming) => self.on_incoming(incoming).await,
                        None => {
                            // Check local state
                            match self.continue_or_stop_by_state() {
                                Running::Continue => Err(Error::Io(io::Error::new(
                                    io::ErrorKind::UnexpectedEof,
                                    "Connection has stopped before session is ended"
                                ))),
                                Running::Stop => Ok(Running::Stop),
                            }

                        }
                    };
                    // tracing::info!(task="on_incoming", result = ?result);
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
                    // tracing::info!(task="on_control", result = ?result);
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
                    // tracing::info!(task="on_outgoing_link_frames", result = ?result);
                    result
                }
            };

            let running = match result {
                Ok(running) => running,
                Err(error) => {
                    error!("{:?}", error);
                    let running = self.on_error(&error).await;
                    outcome = Err(error);
                    running
                }
            };

            match running {
                Running::Continue => {}
                Running::Stop => break,
            }
        }

        debug!("Stopped");
        let _ =
            connection::deallocate_session(&mut self.conn, self.session.outgoing_channel()).await;
        outcome
    }
}

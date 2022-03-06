use std::{io, fmt::Debug};

use fe2o3_amqp_types::{definitions::{AmqpError, self, Handle}, performatives::Detach};
use futures_util::SinkExt;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::PollSender;
use tracing::{debug, error, instrument};

use crate::{
    connection::engine::SessionId,
    control::{ConnectionControl, SessionControl},
    endpoint::{Session, self},
    link::{LinkFrame, LinkHandle},
    util::Running,
};

use super::{Error, SessionFrame, SessionFrameBody, frame::SessionIncomingItem, SessionState, error::AllocLinkError};

pub(crate) struct SessionEngine<S: Session> {
    conn: mpsc::Sender<ConnectionControl>,
    session: S,
    session_id: SessionId,
    control: mpsc::Receiver<SessionControl>,
    incoming: mpsc::Receiver<SessionIncomingItem>,
    outgoing: PollSender<SessionFrame>,

    outgoing_link_frames: mpsc::Receiver<LinkFrame>,
}

impl<S> SessionEngine<S>
where
    S: endpoint::Session<State = SessionState, LinkHandle = LinkHandle> + Send + 'static,
    S::Error: Into<Error> + Debug,
    S::AllocError: Into<AllocLinkError>,
{
    pub async fn begin(
        conn: mpsc::Sender<ConnectionControl>,
        session: S,
        session_id: SessionId,
        control: mpsc::Receiver<SessionControl>,
        incoming: mpsc::Receiver<SessionIncomingItem>,
        outgoing: PollSender<SessionFrame>,
        outgoing_link_frames: mpsc::Receiver<LinkFrame>,
    ) -> Result<Self, Error> {
        let mut engine = Self {
            conn,
            session,
            session_id,
            control,
            incoming,
            outgoing,
            outgoing_link_frames,
        };

        // send a begin
        engine.session.send_begin(&mut engine.outgoing).await
            .map_err(|e| e.into())?;
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
        let remote_begin = match body {
            SessionFrameBody::Begin(begin) => begin,
            SessionFrameBody::End(end) => return Err(Error::Remote(
                end.error.unwrap_or_else(|| definitions::Error::new(
                    AmqpError::InternalError,
                    Some("Remote closed session wihtout explanation".into()),
                    None
                ))
            )),
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)), // End session with illegal state
        };
        engine
            .session
            .on_incoming_begin(channel, remote_begin)
            .await
            .map_err(Into::into)?;
        Ok(engine)
    }

    pub fn spawn(self) -> JoinHandle<Result<(), Error>> {
        tokio::spawn(self.event_loop())
    }

    #[inline]
    async fn on_incoming(&mut self, incoming: SessionIncomingItem) -> Result<Running, Error> {
        let SessionFrame { channel, body } = incoming;

        match body {
            SessionFrameBody::Begin(begin) => {
                self.session.on_incoming_begin(channel, begin).await
                    .map_err(Into::into)?;
            }
            SessionFrameBody::Attach(attach) => {
                self.session.on_incoming_attach(channel, attach).await
                    .map_err(Into::into)?;
            }
            SessionFrameBody::Flow(flow) => {
                self.session.on_incoming_flow(channel, flow).await
                    .map_err(Into::into)?;
            }
            SessionFrameBody::Transfer {
                performative,
                payload,
            } => {
                self.session
                    .on_incoming_transfer(channel, performative, payload)
                    .await
                    .map_err(Into::into)?;
            }
            SessionFrameBody::Disposition(disposition) => {
                self.session
                    .on_incoming_disposition(channel, disposition)
                    .await
                    .map_err(Into::into)?;
            }
            SessionFrameBody::Detach(detach) => {
                self.session.on_incoming_detach(channel, detach).await
                    .map_err(Into::into)?;
            }
            SessionFrameBody::End(end) => {
                self.session.on_incoming_end(channel, end).await
                    .map_err(Into::into)?;
            }
        }

        match self.session.local_state() {
            SessionState::Unmapped => Ok(Running::Stop),
            _ => Ok(Running::Continue),
        }
    }

    #[inline]
    async fn on_control(&mut self, control: SessionControl) -> Result<Running, Error> {
        match control {
            SessionControl::End(error) => {
                self.session.send_end(&mut self.outgoing, error).await
                    .map_err(Into::into)?;
            }
            SessionControl::AllocateLink {
                link_name,
                link_handle,
                responder,
            } => {
                let result = self.session.allocate_link(link_name, link_handle);
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
                let flow = self.session.on_outgoing_flow(flow)
                    .map_err(Into::into)?;
                self.outgoing
                    .send(flow)
                    .await
                    // The receiving half must have dropped, and thus the `Connection`
                    // event loop has stopped. It should be treated as an io error
                    .map_err(|e| Error::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;
            }
            SessionControl::Disposition(disposition) => {
                let disposition = self.session.on_outgoing_disposition(disposition)
                    .map_err(Into::into)?;
                self.outgoing
                    .send(disposition)
                    .await
                    // The receiving half must have dropped, and thus the `Connection`
                    // event loop has stopped. It should be treated as an io error
                    .map_err(|e| Error::Io(io::Error::new(io::ErrorKind::Other, e.to_string())))?;
            }
        }

        match self.session.local_state() {
            SessionState::Unmapped => Ok(Running::Stop),
            _ => Ok(Running::Continue),
        }
    }

    #[inline]
    async fn on_outgoing_link_frames(&mut self, frame: LinkFrame) -> Result<Running, Error> {
        match self.session.local_state() {
            SessionState::Mapped => {}
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)),  // End session with illegal state
        }

        let session_frame = match frame {
            LinkFrame::Attach(attach) => self.session.on_outgoing_attach(attach)
                .map_err(Into::into)?,
            LinkFrame::Flow(flow) => self.session.on_outgoing_flow(flow)
                .map_err(Into::into)?,
            LinkFrame::Transfer {
                performative,
                payload,
            } => self.session.on_outgoing_transfer(performative, payload)
                .map_err(Into::into)?,
            LinkFrame::Disposition(disposition) => {
                self.session.on_outgoing_disposition(disposition)
                    .map_err(Into::into)?
            }
            LinkFrame::Detach(detach) => self.session.on_outgoing_detach(detach)
                .map_err(Into::into)?,
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
                let _ = self.session
                    .send_end(&mut self.outgoing, Some(error.clone())).await;
                self.continue_or_stop_by_state()
            },
            Error::Remote(_) => self.continue_or_stop_by_state(),
            Error::LinkHandleError {
                handle, closed, error
            } => self.on_link_handle_error(handle, closed, error).await
        }
    }

    #[inline]
    async fn on_link_handle_error(&mut self, handle: &Handle, closed: &bool, error: &definitions::Error) -> Running {
        // TODO: detach?
        let detach = Detach {
            handle: handle.clone(),
            closed: closed.clone(),
            error: Some(error.clone()),
        };
        match self.session.on_outgoing_detach(detach) {
            Ok(frame) => {
                if let Err(error) = self.outgoing.send(frame).await {
                    // The connection must have dropped
                    error!(?error);
                    return Running::Stop
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
            SessionState::Unmapped
            | SessionState::Discarding => Running::Stop,
        }
    }

    #[instrument(name = "Session::event_loop", skip(self), fields(outgoing_channel = %self.session.outgoing_channel()))]
    async fn event_loop(mut self) -> Result<(), Error> {
        let mut outcome = Ok(());
        loop {
            let result = tokio::select! {
                incoming = self.incoming.recv() => {
                    match incoming {
                        Some(incoming) => self.on_incoming(incoming).await,
                        None => {
                            // Check local state
                            match self.session.local_state() {
                                SessionState::BeginSent
                                | SessionState::BeginReceived
                                | SessionState::Mapped
                                | SessionState::EndSent
                                | SessionState::EndReceived => {
                                    Err(Error::Io(io::Error::new(
                                        io::ErrorKind::UnexpectedEof,
                                        "Connection has stopped before session is ended"
                                    )))
                                },
                                SessionState::Unmapped
                                | SessionState::Discarding => Ok(Running::Stop),
                            }

                        }
                    }
                },
                control = self.control.recv() => {
                    match control {
                        Some(control) => self.on_control(control).await,
                        None => {
                            // all Links and SessionHandle are dropped
                            Ok(Running::Stop)
                        }
                    }
                },
                frame = self.outgoing_link_frames.recv() => {
                    match frame {
                        Some(frame) => self.on_outgoing_link_frames(frame).await,
                        None => {
                            // all Links and SessionHandle are dropped
                            Ok(Running::Stop)
                        }
                    }
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
        // The `SendError` could only occur when the receiving side has been dropped,
        // meaning the `ConnectionEngine::event_loop` has already stopped. There, then,
        // is no need to remove the channel from `ConnectionEngine`, and we could thus
        // ignore this error
        let _ = self
            .conn
            .send(ConnectionControl::DeallocateSession(self.session_id))
            .await;
        outcome
    }
}

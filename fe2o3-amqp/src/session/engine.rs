use std::io;

use fe2o3_amqp_types::definitions::AmqpError;
use futures_util::SinkExt;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::PollSender;

use crate::{
    connection::engine::SessionId,
    control::{ConnectionControl, SessionControl},
    endpoint,
    link::{LinkFrame, LinkHandle},
    util::Running,
};

use super::{
    AllocLinkError, Error, SessionFrame, SessionFrameBody, SessionIncomingItem, SessionState,
};

pub struct SessionEngine<S> {
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
    S::Error: Into<Error>,
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
        engine
            .session
            .send_begin(&mut engine.outgoing)
            .await
            .map_err(|e| e.into())?;
        // wait for an incoming begin
        let frame = match engine.incoming.recv().await {
            Some(frame) => frame, // Receiver<Result<SessionFrame, EngineError>>
            None => todo!(),
        };
        let SessionFrame { channel, body } = frame;
        let remote_begin = match body {
            SessionFrameBody::Begin(begin) => begin,
            _ => return Err(AmqpError::IllegalState.into()),
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
                self.session
                    .on_incoming_begin(channel, begin)
                    .await
                    .map_err(Into::into)?;
            }
            SessionFrameBody::Attach(attach) => {
                self.session
                    .on_incoming_attach(channel, attach)
                    .await
                    .map_err(Into::into)?;
            }
            SessionFrameBody::Flow(flow) => {
                self.session
                    .on_incoming_flow(channel, flow)
                    .await
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
                self.session
                    .on_incoming_detach(channel, detach)
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
    async fn on_control(&mut self, control: SessionControl) -> Result<Running, Error> {
        match control {
            SessionControl::Begin => {
                self.session
                    .send_begin(&mut self.outgoing)
                    .await
                    .map_err(Into::into)?;
            }
            SessionControl::End(error) => {
                self.session
                    .send_end(&mut self.outgoing, error)
                    .await
                    .map_err(Into::into)?;
            }
            SessionControl::AllocateLink {
                link_handle,
                responder,
            } => {
                let result = self.session.allocate_link(link_handle);
                responder.send(result.map_err(Into::into)).map_err(|_| {
                    Error::Io(io::Error::new(
                        io::ErrorKind::Other,
                        "SessionHandle is dropped",
                    ))
                })?;
            }
            SessionControl::DeallocateLink(handle) => {
                todo!()
            }
            SessionControl::LinkFlow(_) => {
                todo!()
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
            _ => return Err(AmqpError::IllegalState.into()),
        }

        let session_frame = match frame {
            LinkFrame::Attach(attach) => self
                .session
                .on_outgoing_attach(attach)
                .map_err(Into::into)?,
            LinkFrame::Flow(flow) => self.session.on_outgoing_flow(flow).map_err(Into::into)?,
            LinkFrame::Transfer {
                performative,
                payload,
            } => self
                .session
                .on_outgoing_transfer(performative, payload)
                .map_err(Into::into)?,
            LinkFrame::Disposition(disposition) => self
                .session
                .on_outgoing_disposition(disposition)
                .map_err(Into::into)?,
            LinkFrame::Detach(detach) => self
                .session
                .on_outgoing_detach(detach)
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

    async fn event_loop(mut self) -> Result<(), Error> {
        loop {
            let result = tokio::select! {
                incoming = self.incoming.recv() => {
                    match incoming {
                        Some(incoming) => self.on_incoming(incoming).await,
                        None => {
                            // TODO: incoming connection dropped
                            Ok(Running::Stop)
                        }
                    }
                },
                control = self.control.recv() => {
                    match control {
                        Some(control) => self.on_control(control).await,
                        None => todo!()
                    }
                },
                frame = self.outgoing_link_frames.recv() => {
                    match frame {
                        Some(frame) => self.on_outgoing_link_frames(frame).await,
                        None => {
                            // all Links and SessionHandle are dropped
                            todo!()
                        }
                    }
                }
            };

            match result {
                Ok(running) => match running {
                    Running::Continue => {}
                    Running::Stop => break,
                },
                Err(err) => {
                    // TODO: handle errors

                    panic!("{:?}", err)
                }
            }
        }

        println!(">>> Debug: SessionEngine exiting event_loop");
        // The `SendError` could only occur when the receiving side has been dropped,
        // meaning the `ConnectionEngine::event_loop` has already stopped. There, then,
        // is no need to remove the channel from `ConnectionEngine`, and we could thus
        // ignore this error
        let _ = self
            .conn
            .send(ConnectionControl::DeallocateSession(self.session_id))
            .await;
        Ok(())
    }
}

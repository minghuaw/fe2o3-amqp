//! The engine handles incoming and outgoing frames and messages to reduce
//! transferring frames/messages over channels

use std::cmp::min;
use std::time::Duration;

use fe2o3_amqp_types::definitions::ConnectionError;
use fe2o3_amqp_types::performatives::{Attach, Close};
use tokio::io::{AsyncRead, AsyncWrite};
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use tokio::sync::mpsc::{Receiver};
use tokio::task::JoinHandle;

use crate::connection::Connection;
use crate::control::{ConnectionControl, SessionControl};
use crate::session::{SessionFrame, SessionFrameBody};
use crate::util::Running;
use crate::{connection, endpoint};
use crate::error::EngineError;
use crate::transport::Transport;
use crate::transport::amqp::Frame;

use super::ConnectionState;
use super::heartbeat::HeartBeat;


pub struct ConnectionEngine<Io, C> {
    transport: Transport<Io>,
    connection: C,
    control: Receiver<ConnectionControl>,
    outgoing_session_frames: Receiver<SessionFrame>,
    // session_control: Receiver<SessionControl>, 

    heartbeat: HeartBeat,
}

impl<Io, C> ConnectionEngine<Io, C> 
where 
    Io: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    C: endpoint::Connection<State = ConnectionState> + Send + 'static,
{
    pub fn new(
        transport: Transport<Io>, 
        connection: C,
        control: Receiver<ConnectionControl>,
        outgoing_session_frames: Receiver<SessionFrame>,
        // session_control: Receiver<SessionControl>, 
    ) -> Self {
        Self {
            transport,
            connection,
            control,
            // session_control,
            outgoing_session_frames,
            heartbeat: HeartBeat::never(),
        }
    }

    /// Open Connection without starting the Engine::event_loop()
    pub(crate) async fn open(
        transport: Transport<Io>, 
        connection: C,
        control: Receiver<ConnectionControl>,
        outgoing_session_frames: Receiver<SessionFrame>,
        // session_control: Receiver<SessionControl>, 
    ) -> Result<Self, EngineError> {
        use crate::transport::amqp::FrameBody;

        let mut engine = Self {
            transport,
            connection,
            control,
            outgoing_session_frames,
            // session_control,
            heartbeat: HeartBeat::never(),
        };

        // Send an Open
        engine.connection.on_outgoing_open(&mut engine.transport)
            .await
            .map_err(Into::into)?;
        
        // Wait for an Open
        let frame = match engine.transport.next().await {
            Some(frame) => frame?,
            None => todo!()
        };
        let Frame{channel, body} = frame;
        let remote_open = match body {
            FrameBody::Open(open) => open,
            _ => return Err(EngineError::ConnectionError(ConnectionError::FramingError)),
        };

        // Handle incoming remote_open
        let remote_max_frame_size = remote_open.max_frame_size.0;
        let remote_idle_timeout = remote_open.idle_time_out;
        engine.connection.on_incoming_open(channel, remote_open).await
            .map_err(Into::into)?;

        // update transport setting
        let max_frame_size = min(
            engine.connection.local_open().max_frame_size.0,
            remote_max_frame_size
        );
        engine.transport.set_max_frame_size(max_frame_size as usize);

        // Set heartbeat here because in pipelined-open, the Open frame
        // may be recved after mux loop is started
        match &remote_idle_timeout {
            Some(millis) => {
                let period = Duration::from_millis(*millis as u64);
                engine.heartbeat = HeartBeat::new(period);
            }
            None => engine.heartbeat = HeartBeat::never(),
        };

        Ok(engine)
    }

    pub fn spawn(self) -> JoinHandle<Result<(), EngineError>> {
        tokio::spawn(self.event_loop())
    }
}

impl<Io, C> ConnectionEngine<Io, C> 
where 
    Io: AsyncRead + AsyncWrite + Send + Unpin,
    C: endpoint::Connection<State = ConnectionState> + Send + 'static,
{
    async fn forward_to_session(&mut self, channel: u16, frame: SessionFrame) -> Result<(), EngineError> {
        match &self.connection.local_state() {
            ConnectionState::Opened => { },
            _ => return Err(EngineError::illegal_state())
        };

        match self.connection.session_tx_by_incoming_channel(channel) {
            Some(tx) => tx.send(frame).await?,
            None => return Err(EngineError::not_found()),
        };
        Ok(())
    }

    async fn on_incoming(&mut self, incoming: Result<Frame, EngineError>) -> Result<Running, EngineError> {
        use crate::transport::amqp::FrameBody;

        let frame = incoming?;

        let Frame {channel, body} = frame;

        match body {
            FrameBody::Open(open) => {
                let remote_max_frame_size = open.max_frame_size.0;
                let remote_idle_timeout = open.idle_time_out;
                self.connection.on_incoming_open(channel, open).await
                    .map_err(Into::into)?;

                // update transport setting
                let max_frame_size = min(
                    self.connection.local_open().max_frame_size.0,
                    remote_max_frame_size
                );
                self.transport.set_max_frame_size(max_frame_size as usize);

                // Set heartbeat here because in pipelined-open, the Open frame
                // may be recved after mux loop is started
                match &remote_idle_timeout {
                    Some(millis) => {
                        let period = Duration::from_millis(*millis as u64);
                        self.heartbeat = HeartBeat::new(period);
                    }
                    None => self.heartbeat = HeartBeat::never(),
                };
            },
            FrameBody::Begin(begin) => {
                self.connection.on_incoming_begin(channel, begin).await
                    .map_err(Into::into)?;
            },
            FrameBody::Attach(attach) => {
                let sframe = SessionFrame::new(channel, SessionFrameBody::attach(attach));
                self.forward_to_session(channel, sframe).await?;
            },
            FrameBody::Flow(flow) => {
                let sframe = SessionFrame::new(channel, SessionFrameBody::flow(flow));
                self.forward_to_session(channel, sframe).await?;
            },
            FrameBody::Transfer{performative, payload} => {
                let sframe = SessionFrame::new(channel, SessionFrameBody::transfer(performative, payload));
                self.forward_to_session(channel, sframe).await?;
            },
            FrameBody::Disposition(disposition) => {
                let sframe = SessionFrame::new(channel, SessionFrameBody::disposition(disposition));
                self.forward_to_session(channel, sframe).await?;
            },
            FrameBody::Detach(detach) => {
                let sframe = SessionFrame::new(channel, SessionFrameBody::detach(detach));
                self.forward_to_session(channel, sframe).await?;
            },
            FrameBody::End(end) => {
                self.connection.on_incoming_end(channel, end).await
                    .map_err(Into::into)?;
            },
            FrameBody::Close(close) => {
                self.connection.on_incoming_close(channel, close).await
                    .map_err(Into::into)?
            },
            FrameBody::Empty => {
                // do nothing, IdleTimeout is tracked by Transport
            }
        }

        match self.connection.local_state() {
            ConnectionState::End => Ok(Running::Stop),
            _ => Ok(Running::Continue)
        }
    }

    #[inline]
    async fn on_control(&mut self, control: ConnectionControl) -> Result<Running, EngineError> {
        match control {
            ConnectionControl::Open => {
                // let open = self.connection.local_open().clone();
                self.connection.on_outgoing_open(&mut self.transport).await
                    .map_err(Into::into)?;
            },
            ConnectionControl::Close(error) => {
                self.connection.on_outgoing_close(&mut self.transport, error).await
                    .map_err(Into::into)?;
            },
            ConnectionControl::CreateSession{tx, responder} => {
                let (channel, session_id) = self.connection.create_session(tx).map_err(Into::into)?;
                responder.send((channel, session_id)).map_err(|_| EngineError::Message("Oneshot channel dropped"))?;
            },
            ConnectionControl::DropSession(session_id) => self.connection.drop_session(session_id)
        }

        match self.connection.local_state() {
            ConnectionState::End => Ok(Running::Stop),
            _ => Ok(Running::Continue)
        }
    }

    #[inline]
    async fn on_outgoing_session_frames(&mut self, frame: SessionFrame) -> Result<Running, EngineError> {
        use crate::transport::amqp::FrameBody;

        match self.connection.local_state() {
            ConnectionState::Opened => {},
            _ => return Err(EngineError::Message("Illegal local connection state"))
        }

        let SessionFrame { channel, body } = frame;

        let frame = match body {
            SessionFrameBody::Begin(begin) => {
                self.connection.on_outgoing_begin(channel, begin).await 
                    .map_err(Into::into)?
            },
            SessionFrameBody::Attach(attach) => {
                Frame::new(channel, FrameBody::attach(attach))
            },
            SessionFrameBody::Flow(flow) => {
                Frame::new(channel, FrameBody::flow(flow))
            },
            SessionFrameBody::Transfer{performative, payload} => {
                Frame::new(channel, FrameBody::transfer(performative, payload))
            },
            SessionFrameBody::Disposition(disposition) => {
                Frame::new(channel, FrameBody::disposition(disposition))
            },
            SessionFrameBody::Detach(detach) => {
                Frame::new(channel, FrameBody::detach(detach))
            },
            SessionFrameBody::End(end) => {
                self.connection.on_outgoing_end(channel, end).await
                    .map_err(Into::into)?
            }
        };

        self.transport.send(frame).await?;
        Ok(Running::Continue)
    }

    #[inline]
    async fn on_heartbeat(&mut self) -> Result<Running, EngineError> {
        match &self.connection.local_state() {
            ConnectionState::Start 
            | ConnectionState::CloseSent => return Ok(Running::Continue),
            ConnectionState::End => return Ok(Running::Stop),
            _ => {}
        }

        let frame = Frame::empty();
        self.transport.send(frame).await?;
        Ok(Running::Continue)
    }

    async fn event_loop(mut self) -> Result<(), EngineError>{
        loop {
            let result = tokio::select! {
                _ = self.heartbeat.next() => self.on_heartbeat().await,
                incoming = self.transport.next() => {
                    match incoming {
                        Some(incoming) => self.on_incoming(incoming).await,
                        None => todo!(),
                    }
                },
                control = self.control.recv() => {
                    match control {
                        Some(control) => self.on_control(control).await,
                        None => todo!()
                    }
                },
                outgoing = self.outgoing_session_frames.recv() => {
                    match outgoing {
                        Some(frame) => self.on_outgoing_session_frames(frame).await,
                        None => todo!()
                    }
                }
            };

            match result {
                Ok(running) => {
                    match running {
                        Running::Continue => {},
                        Running::Stop => break,
                    }
                },
                Err(err) => {
                    todo!()
                }
            }
        }

        Ok(())
    }
}

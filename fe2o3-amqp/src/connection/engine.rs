//! The engine handles incoming and outgoing frames and messages to reduce
//! transferring frames/messages over channels

use std::cmp::min;
use std::time::Duration;

use fe2o3_amqp_types::definitions::ConnectionError;
use fe2o3_amqp_types::performatives::Close;
use tokio::io::{AsyncRead, AsyncWrite};
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use tokio::sync::mpsc::{Receiver, UnboundedReceiver};
use tokio::task::JoinHandle;

use crate::connection::Connection;
use crate::control::{ConnectionControl, SessionControl};
use crate::session::{SessionFrame, SessionFrameBody};
use crate::{connection, endpoint};
use crate::error::EngineError;
use crate::transport::Transport;
use crate::transport::amqp::Frame;

use super::ConnectionState;
use super::heartbeat::HeartBeat;

pub enum Running {
    Continue,
    Stop,
}

pub struct ConnectionEngine<Io, C> {
    transport: Transport<Io>,
    connection: C,
    connection_control: UnboundedReceiver<ConnectionControl>,
    // session_control: UnboundedReceiver<SessionControl>, 

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
        connection_control: UnboundedReceiver<ConnectionControl>,
        // session_control: UnboundedReceiver<SessionControl>, 
    ) -> Self {
        Self {
            transport,
            connection,
            connection_control,
            // session_control,
            heartbeat: HeartBeat::never(),
        }
    }

    /// Open Connection without starting the Engine::event_loop()
    pub(crate) async fn open(
        transport: Transport<Io>, 
        connection: C,
        connection_control: UnboundedReceiver<ConnectionControl>,
        // session_control: UnboundedReceiver<SessionControl>, 
    ) -> Result<Self, EngineError> {
        use crate::transport::amqp::FrameBody;

        let mut engine = Self {
            transport,
            connection,
            connection_control,
            // session_control,
            heartbeat: HeartBeat::never(),
        };

        // Send an Open
        engine.connection.on_outgoing_open(
            &mut engine.transport, 
            0, 
            engine.connection.local_open().clone())
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
    async fn forward_incoming_session_frame(&mut self, channel: u16, frame: SessionFrame) -> Result<(), EngineError> {
        // match self.connection.session_tx_by_incoming_channel(channel) {
        //     Some(tx) => tx.send(frame),
        //     None => 
        // }

        todo!()
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
            FrameBody::Begin(mut begin) => {
                self.connection.on_incoming_begin(channel, &mut begin).await
                    .map_err(Into::into)?;
                let sframe = SessionFrame::new(channel, SessionFrameBody::begin(begin));
                match self.connection.session_tx_by_incoming_channel(channel) {
                    Some(tx) => tx.send(sframe)?,
                    None => todo!()
                }
            },
            FrameBody::Attach(attach) => {
                todo!()
            },
            FrameBody::Flow(flow) => {
                todo!()
            },
            FrameBody::Transfer{performative, payload} => {
                todo!()
            },
            FrameBody::Disposition(disposition) => {
                todo!()
            },
            FrameBody::Detach(detach) => {
                todo!()
            },
            FrameBody::End(mut end) => {
                self.connection.on_incoming_end(channel, &mut end).await
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
    async fn on_connection_control(&mut self, control: ConnectionControl) -> Result<Running, EngineError> {
        match control {
            ConnectionControl::Open => {
                let open = self.connection.local_open().clone();
                self.connection.on_outgoing_open(&mut self.transport, 0, open).await
                    .map_err(Into::into)?;
            },
            ConnectionControl::Close(error) => {
                let close = Close::new(error);
                self.connection.on_outgoing_close(&mut self.transport, 0, close).await
                    .map_err(Into::into)?;
            }
            ConnectionControl::Begin => {
                todo!()
            }
        }

        match self.connection.local_state() {
            ConnectionState::End => Ok(Running::Stop),
            _ => Ok(Running::Continue)
        }
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
                control = self.connection_control.recv() => {
                    match control {
                        Some(control) => self.on_connection_control(control).await,
                        None => todo!()
                    }
                },
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

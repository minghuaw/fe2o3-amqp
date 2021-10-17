//! The engine handles incoming and outgoing frames and messages to reduce
//! transferring frames/messages over channels

use tokio::io::{AsyncRead, AsyncWrite};
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use tokio::sync::mpsc::Receiver;

use crate::control::{ConnectionControl, SessionControl};
use crate::endpoint;
use crate::error::EngineError;
use crate::transport::Transport;
use crate::transport::amqp::Frame;
use crate::transport::connection::ConnectionState;

pub enum Running {
    Continue,
    Stop,
}

pub struct Engine<Io, C> {
    transport: Transport<Io>,
    connection: C,
    connection_control: Receiver<ConnectionControl>,
    session_control: Receiver<SessionControl>, 
}

impl<Io, C> Engine<Io, C> 
where 
    Io: AsyncRead + AsyncWrite + Send + Unpin,
    C: endpoint::Connection<State = ConnectionState>,
{
    async fn on_incoming(&mut self, incoming: Result<Frame, EngineError>) -> Result<Running, EngineError> {
        use crate::transport::amqp::FrameBody;
        use crate::endpoint::Session;

        let frame = incoming?;

        let Frame {channel, mut body} = frame;

        match body {
            FrameBody::Open(open) => self.connection.on_incoming_open(channel, open).await
                .map_err(Into::into)?,
            FrameBody::Begin(mut begin) => {
                self.connection.on_incoming_begin(channel, &mut begin).await
                    .map_err(Into::into)?;
                self.connection.session_mut_by_incoming_channel(channel)
                    .map_err(Into::into)?
                    .on_incoming_begin(begin).await
                    .map_err(Into::into)?;
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
                self.connection.session_mut_by_incoming_channel(channel)
                    .map_err(Into::into)?
                    .on_incoming_end(end).await
                    .map_err(Into::into)?;
            },
            FrameBody::Close(close) => self.connection.on_incoming_close(channel, close).await
                .map_err(Into::into)?,
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
        
        
        todo!()
    }

    #[inline]
    async fn on_session_control(&mut self, control: SessionControl) -> Result<Running, EngineError> {
        todo!()
    }

    async fn event_loop(mut self) {
        loop {
            let result = tokio::select! {
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
                control = self.session_control.recv() => {
                    match control {
                        Some(control) => self.on_session_control(control).await,
                        None => todo!(),
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
    }
}

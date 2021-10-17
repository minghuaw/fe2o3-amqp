//! The engine handles incoming and outgoing frames and messages to reduce
//! transferring frames/messages over channels

use tokio::io::{AsyncRead, AsyncWrite};
use futures_util::{Sink, SinkExt, Stream, StreamExt};

use crate::endpoint;
use crate::error::EngineError;
use crate::transport::Transport;
use crate::transport::amqp::Frame;

pub struct Engine<Io, C> {
    transport: Transport<Io>,
    connection: C,
}

impl<Io, C> Engine<Io, C> 
where 
    Io: AsyncRead + AsyncWrite + Send + Unpin,
    C: endpoint::Connection,
{
    async fn on_incoming(&mut self, incoming: Result<Frame, EngineError>) -> Result<(), EngineError> {
        use crate::transport::amqp::FrameBody;
        use crate::endpoint::Session;

        let frame = incoming?;

        let Frame {channel, mut body} = frame;

        match &mut body {
            FrameBody::Open(open) => self.connection.on_incoming_open(channel, open).await
                .map_err(Into::into)?,
            FrameBody::Begin(begin) => {
                self.connection.on_incoming_begin(channel, begin).await
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
            FrameBody::End(end) => {
                self.connection.on_incoming_end(channel, end).await
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

        todo!()
    }

    async fn event_loop(mut self) {
        loop {
            let result = tokio::select! {
                incoming = self.transport.next() => {
                    match incoming {
                        Some(incoming) => self.on_incoming(incoming).await,
                        None => break,
                    }
                }
            };
        }
    }
}

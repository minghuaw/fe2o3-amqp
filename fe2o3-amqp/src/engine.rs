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
    C: endpoint::Connection<Error = EngineError>,
{
    async fn on_incoming(&mut self, next: Result<Frame, EngineError>) -> Result<(), EngineError> {
        use crate::transport::amqp::FrameBody;

        let frame = next?;

        let Frame {channel, mut body} = frame;

        match &mut body {
            FrameBody::Open(open) => self.connection.on_incoming_open(channel, open).await?,
            FrameBody::Begin(begin) => self.connection.on_incoming_begin(channel, begin).await?,
        }

        unimplemented!()
    }

    async fn event_loop(mut self) {
        loop {
            let result = tokio::select! {
                next = self.transport.next() => {
                    match next {
                        Some(next) => self.on_incoming(next).await,
                        None => break,
                    }
                }
            };
        }
    }
}

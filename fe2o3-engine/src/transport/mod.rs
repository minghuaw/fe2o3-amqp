//! Implements low level transport framing
//!
//! Idea: Two layer design.
//! layer 0: `tokio_util::codec::LengthDelimited` over `AsyncWrite`
//! layer 1: A custom encoder that implements `tokio_util::codec::Encoder` and takes a Frame as an item
//!
//! Layer 0 should be hidden within the connection and there should be API that provide
//! access to layer 1 for types that implement Encoder

pub const FRAME_TYPE_AMQP: u8 = 0x00;
pub const FRAME_TYPE_SASL: u8 = 0x01;

pub mod amqp;
pub mod protocol_header;
pub mod connection;
pub mod session;
pub mod link;
pub mod endpoint;

/* -------------------------------- Transport ------------------------------- */

use std::{convert::TryFrom, task::Poll};

use bytes::{Bytes, BytesMut};
use futures_util::{Sink, Stream};
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_util::codec::{Decoder, Encoder, Framed, LengthDelimitedCodec, LengthDelimitedCodecError};

use crate::error::EngineError;

use amqp::{Frame, FrameCodec};
use protocol_header::{ProtocolHeader, ProtocolId};

use self::connection::ConnectionState;

pin_project! {
    pub struct Transport<Io> {
        #[pin]
        framed: Framed<Io, LengthDelimitedCodec>
    }
}

impl<Io: AsyncRead + AsyncWrite + Unpin> Transport<Io> {
    pub fn bind(io: Io) -> Result<Self, EngineError> {
        let framed = LengthDelimitedCodec::builder()
            .big_endian()
            .length_field_length(4)
            // Prior to any explicit negotiation, 
            // the maximum frame size is 512 (MIN-MAX-FRAME-SIZE)
            .max_frame_length(512) // change max frame size later in negotiation
            .length_adjustment(-4)
            .new_framed(io);
        Ok(Self { framed })
    }

    pub async fn negotiate(
        io: &mut Io,
        local_state: &mut ConnectionState,
        proto_header: ProtocolHeader,
    ) -> Result<ProtocolHeader, EngineError> {
        // negotiation
        let outbound_buf: [u8; 8] = proto_header.clone().into();
        io.write_all(&outbound_buf).await?;

        // State transition
        *local_state = ConnectionState::HeaderSent;

        // wait for incoming header
        let mut inbound_buf = [0u8; 8];
        io.read_exact(&mut inbound_buf).await?;

        // State transition
        *local_state = ConnectionState::HeaderExchange;

        // check header
        let incoming_header = ProtocolHeader::try_from(inbound_buf)?;
        if incoming_header != proto_header {
            return Err(EngineError::UnexpectedProtocolHeader(inbound_buf));
        }
        Ok(incoming_header)
    }

    pub fn set_max_frame_size(&mut self, max_frame_size: usize) -> &mut Self {
        self.framed.codec_mut().set_max_frame_length(max_frame_size);
        self
    }
}

impl<Io> Sink<Frame> for Transport<Io>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    type Error = EngineError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed.poll_ready(cx).map_err(Into::into)
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: Frame) -> Result<(), Self::Error> {
        let mut bytesmut = BytesMut::new();
        let mut encoder = FrameCodec {};
        encoder.encode(item, &mut bytesmut)?;

        let this = self.project();
        this.framed
            .start_send(Bytes::from(bytesmut))
            .map_err(Into::into)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed.poll_flush(cx).map_err(Into::into)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed.poll_close(cx).map_err(Into::into)
    }
}

impl<Io> Stream for Transport<Io> 
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    type Item = Result<Frame, EngineError>;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        match this.framed.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(next) => {
                match next {
                    Some(item) => {
                        let mut src = match item {
                            Ok(b) => b,
                            Err(err) => {
                                use std::any::Any;
                                let any = &err as &dyn Any;
                                if any.is::<LengthDelimitedCodecError>() {
                                    // This should be the only error type
                                    return Poll::Ready(Some(Err(EngineError::MaxFrameSizeExceeded)))
                                } else {
                                    return Poll::Ready(Some(Err(err.into())))
                                }
                            }
                        };
                        let mut decoder = FrameCodec { };
                        Poll::Ready(decoder.decode(&mut src).transpose())
                    },
                    None => Poll::Ready(None)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures_util::{SinkExt, StreamExt};
    use tokio_util::codec::LengthDelimitedCodec;

    #[tokio::test]
    async fn test_length_delimited_codec() {
        // test write
        let mut writer = vec![];
        let mut framed = LengthDelimitedCodec::builder()
            .big_endian()
            .length_field_length(4)
            .length_adjustment(-4)
            .new_write(&mut writer);

        let payload = Bytes::from("AMQP");
        framed.send(payload).await.unwrap();
        println!("{:?}", writer);

        // test read
        let reader = &writer[..];
        let mut framed = LengthDelimitedCodec::builder()
            .big_endian()
            .length_field_length(4)
            .length_adjustment(-4)
            .new_read(reader);
        let outcome = framed.next().await.unwrap();
        println!("{:?}", outcome)
    }
}

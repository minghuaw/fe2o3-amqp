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
use futures_util::{Sink, Stream, StreamExt};
use pin_project_lite::pin_project;
use tokio::{io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf}};
use tokio_util::codec::{Decoder, Encoder, Framed, FramedRead, FramedWrite, LengthDelimitedCodec, LengthDelimitedCodecError};

use crate::error::EngineError;

use amqp::{Frame, FrameCodec};
use protocol_header::{ProtocolHeader, ProtocolId};

use self::connection::ConnectionState;

pin_project! {
    pub struct Transport<Io> {
        #[pin]
        framed_read: FramedRead<ReadHalf<Io>, LengthDelimitedCodec>,
        #[pin]
        framed_write: FramedWrite<WriteHalf<Io>, LengthDelimitedCodec>,

    }
}

impl<Io> Transport<Io> 
where 
    Io: AsyncRead + AsyncWrite + Unpin,
{
    pub fn bind(io: Io) -> Self {
        let (reader, writer) = tokio::io::split(io);

        let framed_write = LengthDelimitedCodec::builder()
            .big_endian()
            .length_field_length(4)
            // Prior to any explicit negotiation, 
            // the maximum frame size is 512 (MIN-MAX-FRAME-SIZE)
            .max_frame_length(512) // change max frame size later in negotiation
            .length_adjustment(-4)
            .new_write(writer);

        let framed_read = LengthDelimitedCodec::builder()
            .big_endian()
            .length_field_length(4)
            // Prior to any explicit negotiation, 
            // the maximum frame size is 512 (MIN-MAX-FRAME-SIZE)
            .max_frame_length(512) // change max frame size later in negotiation
            .length_adjustment(-4)
            .new_read(reader);
        Self { framed_read, framed_write }
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
        self.framed_read.decoder_mut().set_max_frame_length(max_frame_size);
        self.framed_write.encoder_mut().set_max_frame_length(max_frame_size);
        self
    }
}

impl<Io> Sink<Frame> for Transport<Io>
where
    Io: AsyncWrite + Unpin,
{
    type Error = EngineError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed_write.poll_ready(cx).map_err(Into::into)
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: Frame) -> Result<(), Self::Error> {
        let mut bytesmut = BytesMut::new();
        let mut encoder = FrameCodec {};
        encoder.encode(item, &mut bytesmut)?;

        println!(">>> Debug start_send() encoded: {:?}", &bytesmut);

        let this = self.project();
        this.framed_write
            .start_send(Bytes::from(bytesmut))
            .map_err(Into::into)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed_write.poll_flush(cx).map_err(Into::into)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed_write.poll_close(cx).map_err(Into::into)
    }
}

impl<Io> Stream for Transport<Io> 
where
    Io: AsyncRead + Unpin,
{
    type Item = Result<Frame, EngineError>;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        match this.framed_read.poll_next(cx) {
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
    use std::io::Cursor;

    use bytes::Bytes;
    use fe2o3_types::performatives::Open;
    use futures_util::{SinkExt, StreamExt};
    use tokio_util::codec::LengthDelimitedCodec;
    use tokio_test::io::Builder;

    use super::{Transport, amqp::{FrameBody, Frame}, connection::ConnectionState, protocol_header::ProtocolHeader};

    #[tokio::test]
    async fn test_length_delimited_codec() {
        // test write
        let mut writer = vec![];
        let mut framed = LengthDelimitedCodec::builder()
            .big_endian()
            .length_field_length(4)
            // Prior to any explicit negotiation, 
            // the maximum frame size is 512 (MIN-MAX-FRAME-SIZE)
            .max_frame_length(512) // change max frame size later in negotiation
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

    #[tokio::test]
    async fn test_header_exchange() {
        let mut mock = Builder::new()
            .write(b"AMQP")
            .write(&[0, 1, 0, 0])
            .read(b"AMQP")
            .read(&[0, 1, 0, 0])
            .build();

        let mut local_state = ConnectionState::Start;
        Transport::negotiate(&mut mock, &mut local_state, ProtocolHeader::amqp()).await
            .unwrap();
    }

    #[tokio::test]
    async fn test_frame_sink() {
        let mut buf = Vec::new();
        let io = Cursor::new(&mut buf);        
        let mut transport = Transport::bind(io);

        let open = Open{
            container_id: "1234".into(),
            hostname: Some("127.0.0.1".into()), 
            max_frame_size: 100.into(),
            channel_max: 9.into(),
            idle_time_out: Some(10),
            outgoing_locales: None,
            incoming_locales: None,
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None
        };

        let body = FrameBody::Open {
            performative: open
        };
        let frame = Frame::new(0u16, body);

        transport.send(frame).await.unwrap();
        println!("{:x?}", buf);
    }
}

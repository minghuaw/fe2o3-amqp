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
pub mod connection;
pub mod endpoint;
pub mod link;
pub mod protocol_header;
pub mod session;

/* -------------------------------- Transport ------------------------------- */

use std::{convert::TryFrom, task::Poll, time::Duration};

use bytes::{Bytes, BytesMut};
use futures_util::{Future, Sink, Stream};
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_util::codec::{
    Decoder, Encoder, Framed, LengthDelimitedCodec, LengthDelimitedCodecError,
};

use crate::{error::EngineError, util::IdleTimeout};

use amqp::{Frame, FrameCodec};
use protocol_header::ProtocolHeader;

use self::connection::ConnectionState;

pin_project! {
    pub struct Transport<Io> {
        #[pin]
        framed: Framed<Io, LengthDelimitedCodec>,
        #[pin]
        idle_timeout: Option<IdleTimeout>
    }
}

impl<Io> Transport<Io>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    pub fn bind(io: Io, max_frame_size: usize, idle_timeout: Option<Duration>) -> Self {
        let framed = LengthDelimitedCodec::builder()
            .big_endian()
            .length_field_length(4)
            // Prior to any explicit negotiation,
            // the maximum frame size is 512 (MIN-MAX-FRAME-SIZE)
            .max_frame_length(max_frame_size) // change max frame size later in negotiation
            .length_adjustment(-4)
            .new_framed(io);
        let idle_timeout = match idle_timeout {
            Some(duration) => match duration.is_zero() {
                true => None,
                false => Some(IdleTimeout::new(duration)),
            },
            None => None,
        };

        Self {
            framed,
            idle_timeout,
        }
    }
    pub async fn send_proto_header(
        io: &mut Io,
        local_state: &mut ConnectionState,
        proto_header: ProtocolHeader,
    ) -> Result<(), EngineError> {
        let buf: [u8; 8] = proto_header.into();
        match local_state {
            ConnectionState::Start => {
                io.write_all(&buf).await?;
                *local_state = ConnectionState::HeaderSent;
            }
            ConnectionState::HeaderReceived => {
                io.write_all(&buf).await?;
                *local_state = ConnectionState::HeaderExchange
            }
            s @ _ => return Err(EngineError::UnexpectedConnectionState(s.clone())),
        }
        Ok(())
    }

    pub async fn recv_proto_header(
        io: &mut Io,
        local_state: &mut ConnectionState,
        proto_header: ProtocolHeader,
    ) -> Result<ProtocolHeader, EngineError> {
        // wait for incoming header
        match local_state {
            ConnectionState::Start => {
                let incoming_header =
                    read_and_compare_proto_header(io, local_state, &proto_header).await?;
                *local_state = ConnectionState::HeaderReceived;
                Ok(incoming_header)
            }
            ConnectionState::HeaderSent => {
                let incoming_header =
                    read_and_compare_proto_header(io, local_state, &proto_header).await?;
                *local_state = ConnectionState::HeaderExchange;
                Ok(incoming_header)
            }
            s @ _ => return Err(EngineError::UnexpectedConnectionState(s.clone())),
        }
    }

    pub async fn negotiate(
        io: &mut Io,
        local_state: &mut ConnectionState,
        proto_header: ProtocolHeader,
    ) -> Result<ProtocolHeader, EngineError> {
        Self::send_proto_header(io, local_state, proto_header.clone()).await?;
        let incoming_header = Self::recv_proto_header(io, local_state, proto_header).await?;

        Ok(incoming_header)
    }

    pub fn set_max_frame_size(&mut self, max_frame_size: usize) -> &mut Self {
        self.framed.codec_mut().set_max_frame_length(max_frame_size);
        self
    }

    pub fn idle_timeout(&self) -> &Option<IdleTimeout> {
        &self.idle_timeout
    }

    pub fn idle_timeout_mut(&mut self) -> &mut Option<IdleTimeout> {
        &mut self.idle_timeout
    }

    pub fn set_idle_timeout(&mut self, duration: Duration) -> &mut Self {
        let idle_timeout = match duration.is_zero() {
            true => None,
            false => Some(IdleTimeout::new(duration)),
        };

        self.idle_timeout = idle_timeout;
        self
    }
}

async fn read_and_compare_proto_header<Io>(
    io: &mut Io,
    local_state: &mut ConnectionState,
    proto_header: &ProtocolHeader,
) -> Result<ProtocolHeader, EngineError>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    let mut inbound_buf = [0u8; 8];
    io.read_exact(&mut inbound_buf).await?;
    // check header
    let incoming_header = ProtocolHeader::try_from(inbound_buf)?;
    if incoming_header != *proto_header {
        *local_state = ConnectionState::End;
        return Err(EngineError::UnexpectedProtocolHeader(inbound_buf));
    }
    Ok(incoming_header)
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
    Io: AsyncRead + Unpin,
{
    type Item = Result<Frame, EngineError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();

        // First poll codec
        match this.framed.poll_next(cx) {
            Poll::Ready(next) => {
                if let Some(mut delay) = this.idle_timeout.as_pin_mut() {
                    println!(">>> Debug: poll_next() resetting idle_timeout");
                    delay.reset();
                }

                match next {
                    Some(item) => {
                        let mut src = match item {
                            Ok(b) => b,
                            Err(err) => {
                                use std::any::Any;
                                let any = &err as &dyn Any;
                                if any.is::<LengthDelimitedCodecError>() {
                                    // This should be the only error type
                                    return Poll::Ready(Some(Err(
                                        EngineError::MaxFrameSizeExceeded,
                                    )));
                                } else {
                                    return Poll::Ready(Some(Err(err.into())));
                                }
                            }
                        };
                        let mut decoder = FrameCodec {};
                        Poll::Ready(decoder.decode(&mut src).transpose())
                    }
                    None => Poll::Ready(None),
                }
            }
            Poll::Pending => {
                // check if idle timeout has exceeded
                if let Some(delay) = this.idle_timeout.as_pin_mut() {
                    match delay.poll(cx) {
                        Poll::Ready(()) => return Poll::Ready(Some(Err(EngineError::IdleTimeout))),
                        Poll::Pending => return Poll::Pending,
                    }
                }

                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use fe2o3_types::performatives::Open;
    use futures_util::{SinkExt, StreamExt};
    use tokio_test::io::Builder;
    use tokio_util::codec::LengthDelimitedCodec;

    use super::{
        amqp::{Frame, FrameBody},
        connection::ConnectionState,
        protocol_header::ProtocolHeader,
        Transport,
    };

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
        Transport::negotiate(&mut mock, &mut local_state, ProtocolHeader::amqp())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_empty_frame_with_length_delimited_codec() {
        let mock = Builder::new()
            .write(&[0x00, 0x00, 0x00, 0x08]) // size of the frame
            .write(&[0x02, 0x00, 0x00, 0x00])
            .build();
        let mut transport = Transport::bind(mock, 512, None);
        let frame = Frame::empty();
        transport.send(frame).await.unwrap();
    }

    #[tokio::test]
    async fn test_frame_sink() {
        // use std::io::Cursor;
        // let mut buf = Vec::new();
        // let io = Cursor::new(&mut buf);
        // let mut transport = Transport::bind(io);

        let mock = tokio_test::io::Builder::new()
            .write(&[0x0, 0x0, 0x0, 0x29])
            .write(&[0x02, 0x0, 0x0, 0x0])
            .write(&[
                0x00, 0x53, 0x10, 0xC0, 0x1c, 0x05, 0xA1, 0x04, 0x31, 0x32, 0x33, 0x34, 0xA1, 0x09,
                0x31, 0x32, 0x37, 0x2E, 0x30, 0x2E, 0x30, 0x2E, 0x31, 0x70, 0x00, 0x00, 0x03, 0xe8,
                0x60, 0x00, 0x09, 0x52, 0x05,
            ])
            .build();
        let mut transport = Transport::bind(mock, 1000, None);

        let open = Open {
            container_id: "1234".into(),
            hostname: Some("127.0.0.1".into()),
            max_frame_size: 1000.into(),
            channel_max: 9.into(),
            idle_time_out: Some(5),
            outgoing_locales: None,
            incoming_locales: None,
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None,
        };

        let body = FrameBody::Open { performative: open };
        let frame = Frame::new(0u16, body);

        transport.send(frame).await.unwrap();
        // println!("{:x?}", buf);
    }
}

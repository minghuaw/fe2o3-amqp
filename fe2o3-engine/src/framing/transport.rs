use std::{convert::TryFrom, task::Poll};

use bytes::{Bytes, BytesMut};
use fe2o3_types::performatives::MaxFrameSize;
use futures_util::{Sink, Stream};
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_util::codec::{Decoder, Encoder, Framed, LengthDelimitedCodec};

use crate::error::EngineError;

use super::{amqp::{AmqpFrame, AmqpFrameDecoder, AmqpFrameEncoder}, protocol_header::{ProtocolHeader, ProtocolId}};

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
            .max_frame_length(usize::from(MaxFrameSize::default())) // change max frame size later in negotiation
            .length_adjustment(-4)
            .new_framed(io);
        Ok(Self { framed })
    }

    pub async fn negotiate(
        io: &mut Io,
        proto_header: ProtocolHeader,
    ) -> Result<ProtocolId, EngineError> {
        // negotiation
        let outbound_buf: [u8; 8] = proto_header.clone().into();
        io.write_all(&outbound_buf).await?;

        // wait for incoming header
        let mut inbound_buf = [0u8; 8];
        io.read_exact(&mut inbound_buf).await?;

        // check header
        let incoming_header = ProtocolHeader::try_from(inbound_buf)?;
        if incoming_header != proto_header {
            return Err(EngineError::UnexpectedProtocolHeader(inbound_buf));
        }
        Ok(incoming_header.id)
    }

    pub async fn negotiate_and_bind(
        mut io: Io,
        proto_header: ProtocolHeader,
    ) -> Result<Self, EngineError> {
        // bind transport based on proto_id
        match Self::negotiate(&mut io, proto_header).await? {
            ProtocolId::Amqp => Self::bind(io),
            ProtocolId::Tls => todo!(),
            ProtocolId::Sasl => todo!(),
        }
    }

    pub fn set_max_frame_size(&mut self, max_frame_size: usize) -> &mut Self {
        self.framed.codec_mut().set_max_frame_length(max_frame_size);
        self
    }
}

impl<Io> Sink<AmqpFrame> for Transport<Io>
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

    fn start_send(self: std::pin::Pin<&mut Self>, item: AmqpFrame) -> Result<(), Self::Error> {
        let mut bytesmut = BytesMut::new();
        let mut encoder = AmqpFrameEncoder {};
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
    type Item = Result<AmqpFrame, EngineError>;

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        match this.framed.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(next) => {
                match next {
                    Some(item) => {
                        let mut src = match item {
                            Ok(b) => b,
                            Err(err) => return Poll::Ready(Some(Err(err.into())))
                        };
                        let mut decoder = AmqpFrameDecoder { };
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

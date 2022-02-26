//! Implements low level transport framing
//!
//! Idea: Two layer design.
//! layer 0: `tokio_util::codec::LengthDelimited` over `AsyncWrite`
//! layer 1: A custom encoder that implements `tokio_util::codec::Encoder` and takes a Frame as an item
//!
//! Layer 0 should be hidden within the connection and there should be API that provide
//! access to layer 1 for types that implement Encoder

pub(crate) mod error;
pub mod protocol_header;
pub use error::Error;
use fe2o3_amqp_types::{
    definitions::{AmqpError, MIN_MAX_FRAME_SIZE},
    sasl::SaslCode,
};
use tokio_rustls::{client::TlsStream, TlsConnector};
use tracing::{event, instrument, span, trace, Level};

/* -------------------------------- Transport ------------------------------- */

use std::{convert::TryFrom, io, marker::PhantomData, task::Poll, time::Duration};

use bytes::BytesMut;
use futures_util::{Future, Sink, SinkExt, Stream, StreamExt};
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio_util::codec::{
    Decoder, Encoder, Framed, LengthDelimitedCodec, LengthDelimitedCodecError,
};

use crate::{
    connection::ConnectionState,
    frames::{amqp, sasl},
    sasl_profile::{Negotiation, SaslProfile},
    util::IdleTimeout,
};

use protocol_header::ProtocolHeader;

use self::error::NegotiationError;

pin_project! {
    /// Frame transport
    #[derive(Debug)]
    pub struct Transport<Io, Ftype> {
        #[pin]
        framed: Framed<Io, LengthDelimitedCodec>,
        #[pin]
        idle_timeout: Option<IdleTimeout>,
        // frame type
        ftype: PhantomData<Ftype>,
    }
}

impl<Io, Ftype> Transport<Io, Ftype>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    /// Consume the transport and return the wrapped IO
    pub fn into_inner_io(self) -> Io {
        self.framed.into_inner()
    }

    /// Bind to an IO
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
            ftype: PhantomData,
        }
    }
}

impl<Io> Transport<Io, amqp::Frame>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    /// Perform TLS negotiation
    #[instrument(skip_all, fields(domain = %domain))]
    pub async fn connect_tls(
        mut stream: Io,
        domain: &str,
        config: rustls::ClientConfig,
    ) -> Result<TlsStream<Io>, NegotiationError> {
        use rustls::ServerName;
        use std::sync::Arc;

        let span = span!(Level::TRACE, "SEND");
        // Exchange TLS proto header
        let proto_header = ProtocolHeader::tls();
        event!(parent: &span, Level::TRACE, ?proto_header);

        let mut buf: [u8; 8] = proto_header.clone().into();
        stream.write_all(&buf).await?;
        stream.read_exact(&mut buf).await?;

        let span = span!(Level::TRACE, "RECV");
        let incoming_header = match ProtocolHeader::try_from(buf) {
            Ok(header) => {
                event!(parent: &span, Level::TRACE, proto_header = ?header);
                header
            }
            Err(buf) => {
                event!(parent: &span, Level::ERROR, ?buf);
                return Err(NegotiationError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Invalid protocol header",
                )));
            }
        };
        if proto_header != incoming_header {
            return Err(NegotiationError::ProtocolHeaderMismatch(
                incoming_header.into(),
            ));
        }

        // TLS negotiation
        let connector = TlsConnector::from(Arc::new(config));
        let domain = ServerName::try_from(domain).map_err(|_| NegotiationError::InvalidDomain)?;
        let tls = connector.connect(domain, stream).await?;
        Ok(tls)
    }

    /// Performs SASL negotiation
    #[instrument(skip_all, fields(hostname = ?hostname))]
    pub async fn connect_sasl(
        mut stream: Io,
        hostname: Option<&str>,
        mut profile: SaslProfile,
    ) -> Result<Io, NegotiationError> {
        let span = span!(Level::TRACE, "SEND");
        let proto_header = ProtocolHeader::sasl();
        event!(parent: &span, Level::TRACE, ?proto_header);

        let mut buf: [u8; 8] = proto_header.clone().into();
        stream.write_all(&buf).await?;
        stream.read_exact(&mut buf).await?;

        let span = span!(Level::TRACE, "RECV");
        let incoming_header = match ProtocolHeader::try_from(buf) {
            Ok(header) => {
                event!(parent: &span, Level::TRACE, proto_header = ?header);
                header
            }
            Err(buf) => {
                event!(parent: &span, Level::ERROR, ?buf);
                return Err(NegotiationError::ProtocolHeaderMismatch(buf));
            }
        };

        if proto_header != incoming_header {
            return Err(NegotiationError::ProtocolHeaderMismatch(
                incoming_header.into(),
            ));
        }

        // TODO: use a different frame size?
        let mut transport = Transport::<_, sasl::Frame>::bind(stream, MIN_MAX_FRAME_SIZE, None);

        // TODO: timeout?
        while let Some(frame) = transport.next().await {
            let frame = frame?;
            let negotiation = profile.on_frame(frame, hostname).await?;

            match negotiation {
                Negotiation::Continue => {}
                Negotiation::Init(init) => transport.send(sasl::Frame::Init(init)).await?,
                Negotiation::Outcome(outcome) => match outcome.code {
                    SaslCode::Ok => return Ok(transport.into_inner_io()),
                    code @ _ => {
                        return Err(NegotiationError::SaslError {
                            code,
                            additional_data: outcome.additional_data,
                        })
                    }
                },
            }
        }
        Err(NegotiationError::Io(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Expecting SASL negotiation",
        )))
    }

    /// Performs AMQP negotiation
    #[instrument(skip_all)]
    pub async fn negotiate(
        io: &mut Io,
        local_state: &mut ConnectionState,
        proto_header: ProtocolHeader,
    ) -> Result<ProtocolHeader, NegotiationError> {
        send_proto_header(io, local_state, proto_header.clone()).await?;
        let incoming_header = recv_proto_header(io, local_state, proto_header).await?;

        Ok(incoming_header)
    }

    /// Change the max_frame_size for the transport
    pub fn set_max_frame_size(&mut self, max_frame_size: usize) -> &mut Self {
        self.framed.codec_mut().set_max_frame_length(max_frame_size);
        self
    }

    /// Get the idle timeout of the transport
    pub fn idle_timeout(&self) -> &Option<IdleTimeout> {
        &self.idle_timeout
    }

    /// Set the idle timeout of the transport
    pub fn set_idle_timeout(&mut self, duration: Duration) -> &mut Self {
        let idle_timeout = match duration.is_zero() {
            true => None,
            false => Some(IdleTimeout::new(duration)),
        };

        self.idle_timeout = idle_timeout;
        self
    }
}

#[instrument(name = "SEND", skip_all)]
async fn send_proto_header<Io>(
    io: &mut Io,
    local_state: &mut ConnectionState,
    proto_header: ProtocolHeader,
) -> Result<(), NegotiationError>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    trace!(?proto_header);
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
        _ => return Err(NegotiationError::IllegalState), // TODO: is this necessary?
    }
    Ok(())
}

#[instrument(name = "RECV", skip_all)]
async fn recv_proto_header<Io>(
    io: &mut Io,
    local_state: &mut ConnectionState,
    proto_header: ProtocolHeader,
) -> Result<ProtocolHeader, NegotiationError>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    // wait for incoming header
    let proto_header = match local_state {
        ConnectionState::Start => {
            let incoming_header =
                read_and_compare_proto_header(io, local_state, &proto_header).await?;
            *local_state = ConnectionState::HeaderReceived;
            incoming_header
        }
        ConnectionState::HeaderSent => {
            let incoming_header =
                read_and_compare_proto_header(io, local_state, &proto_header).await?;
            *local_state = ConnectionState::HeaderExchange;
            incoming_header
        }
        _ => return Err(NegotiationError::IllegalState),
    };
    trace!(?proto_header);

    Ok(proto_header)
}

async fn read_and_compare_proto_header<Io>(
    io: &mut Io,
    local_state: &mut ConnectionState,
    proto_header: &ProtocolHeader,
) -> Result<ProtocolHeader, NegotiationError>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    let mut inbound_buf = [0u8; 8];
    io.read_exact(&mut inbound_buf).await?;

    // check header
    let incoming_header = match ProtocolHeader::try_from(inbound_buf) {
        Ok(h) => h,
        Err(buf) => {
            tracing::error!(?buf);

            return Err(NegotiationError::NotImplemented(Some(format!("Found: {:?}", inbound_buf))));
        }
    };
    // .map_err(|_| Error::amqp_error(AmqpError::NotImplemented, Some(format!("Found: {:?}", inbound_buf))))?;
    if incoming_header != *proto_header {
        *local_state = ConnectionState::End;
        return Err(NegotiationError::NotImplemented(
            Some(format!(
                "Expecting {:?}, found {:?}",
                proto_header, incoming_header
            )),
        ));
    }
    Ok(incoming_header)
}

impl<Io> Sink<amqp::Frame> for Transport<Io, amqp::Frame>
where
    Io: AsyncWrite + Unpin,
{
    type Error = Error;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed
            .poll_ready(cx) // Result<_, std::io::Error>
            .map_err(Into::into)
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: amqp::Frame) -> Result<(), Self::Error> {
        let mut bytesmut = BytesMut::new();
        let mut encoder = amqp::FrameCodec {};
        encoder.encode(item, &mut bytesmut)?;

        let this = self.project();
        this.framed
            .start_send(bytesmut.freeze()) // Result<_, std::io::Error>
            .map_err(Into::into)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed
            .poll_flush(cx) // Result<_, std::io::Error>
            .map_err(Into::into)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed
            .poll_close(cx) // Result<_, std::io::Error>
            .map_err(Into::into)
    }
}

impl<Io> Stream for Transport<Io, amqp::Frame>
where
    Io: AsyncRead + Unpin,
{
    type Item = Result<amqp::Frame, Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();

        // First poll codec
        match this.framed.poll_next(cx) {
            Poll::Ready(next) => {
                if let Some(mut delay) = this.idle_timeout.as_pin_mut() {
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
                                    return Poll::Ready(Some(Err(Error::amqp_error(
                                        AmqpError::FrameSizeTooSmall,
                                        None,
                                    ))));
                                } else {
                                    return Poll::Ready(Some(Err(err.into())));
                                }
                            }
                        };
                        let mut decoder = amqp::FrameCodec {};
                        Poll::Ready(decoder.decode(&mut src).map_err(Into::into).transpose())
                    }
                    None => Poll::Ready(None),
                }
            }
            Poll::Pending => {
                // check if idle timeout has exceeded
                if let Some(delay) = this.idle_timeout.as_pin_mut() {
                    match delay.poll(cx) {
                        Poll::Ready(()) => return Poll::Ready(Some(Err(Error::IdleTimeout))),
                        Poll::Pending => return Poll::Pending,
                    }
                }

                Poll::Pending
            }
        }
    }
}

impl<Io> Sink<sasl::Frame> for Transport<Io, sasl::Frame>
where
    Io: AsyncWrite + Unpin,
{
    type Error = NegotiationError;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed.poll_ready(cx).map_err(Into::into)
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: sasl::Frame) -> Result<(), Self::Error> {
        // Needs to know the length, and thus cannot write directly to the IO
        let mut bytesmut = BytesMut::new();
        let mut encoder = sasl::FrameCodec {};
        encoder.encode(item, &mut bytesmut)?;

        let this = self.project();
        this.framed
            .start_send(bytesmut.freeze())
            .map_err(Into::into)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed.poll_flush(cx).map_err(Into::into)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed.poll_close(cx).map_err(Into::into)
    }
}

impl<Io> Stream for Transport<Io, sasl::Frame>
where
    Io: AsyncRead + Unpin,
{
    type Item = Result<sasl::Frame, NegotiationError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.framed.poll_next(cx) {
            Poll::Ready(next) => match next {
                Some(item) => {
                    let mut src = match item {
                        Ok(b) => b,
                        Err(err) => {
                            return Poll::Ready(Some(Err(err.into())));
                        }
                    };
                    let mut decoder = sasl::FrameCodec {};
                    Poll::Ready(decoder.decode(&mut src).map_err(Into::into).transpose())
                }
                None => Poll::Ready(None),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use fe2o3_amqp_types::performatives::Open;
    use futures_util::{SinkExt, StreamExt};
    use tokio_test::io::Builder;
    use tokio_util::codec::LengthDelimitedCodec;

    use crate::connection::ConnectionState;

    use super::{
        amqp::{Frame, FrameBody},
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

        // test read
        let reader = &writer[..];
        let mut framed = LengthDelimitedCodec::builder()
            .big_endian()
            .length_field_length(4)
            .length_adjustment(-4)
            .new_read(reader);
        let _outcome = framed.next().await.unwrap();
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

        let body = FrameBody::Open(open);
        let frame = Frame::new(0u16, body);

        transport.send(frame).await.unwrap();
    }
}

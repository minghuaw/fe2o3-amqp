//! Implements low level transport framing
//!
//! Idea: Two layer design.
//! layer 0: `tokio_util::codec::LengthDelimited` over `AsyncWrite`
//! layer 1: A custom encoder that implements `tokio_util::codec::Encoder` and takes a Frame as an item
//!
//! Layer 0 should be hidden within the connection and there should be API that provide
//! access to layer 1 for types that implement Encoder

/* -------------------------------- Transport ------------------------------- */

use fe2o3_amqp_types::{
    definitions::{MAJOR, MINOR, MIN_MAX_FRAME_SIZE, REVISION},
    states::ConnectionState,
};

use std::{io, marker::PhantomData, task::Poll, time::Duration};

use bytes::BytesMut;
use futures_util::{Future, Sink, SinkExt, Stream, StreamExt};
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf};
use tokio_util::codec::{Decoder, Encoder, FramedRead, FramedWrite, LengthDelimitedCodec};

use crate::{
    frames::{amqp, sasl},
    util::IdleTimeout,
};

use protocol_header::ProtocolHeader;

use self::{error::NegotiationError, protocol_header::ProtocolHeaderCodec};

pub(crate) mod error;
pub use error::Error;
pub mod protocol_header;

pin_project! {
    /// Frame transport
    #[derive(Debug)]
    pub struct Transport<Io, Ftype> {
        #[pin]
        framed_write: FramedWrite<WriteHalf<Io>, LengthDelimitedCodec>,

        #[pin]
        framed_read: FramedRead<ReadHalf<Io>, LengthDelimitedCodec>,

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
    /// Consume the transport and return the underlying codec
    pub fn into_framed_codec(
        self,
    ) -> (
        FramedWrite<WriteHalf<Io>, LengthDelimitedCodec>,
        FramedRead<ReadHalf<Io>, LengthDelimitedCodec>,
    ) {
        (self.framed_write, self.framed_read)
    }

    /// Bind to an IO
    pub fn bind(io: Io, max_frame_size: usize, idle_timeout: Option<Duration>) -> Self {
        let (reader, writer) = tokio::io::split(io);
        let encoder = length_delimited_encoder(max_frame_size);
        let framed_write = FramedWrite::new(writer, encoder);

        let decoder = length_delimited_decoder(max_frame_size);
        let framed_read = FramedRead::new(reader, decoder);

        Self::bind_to_framed_codec(framed_write, framed_read, idle_timeout)
    }

    /// Bind transport a framed codec
    pub fn bind_to_framed_codec(
        // framed: Framed<Io, LengthDelimitedCodec>,
        framed_write: FramedWrite<WriteHalf<Io>, LengthDelimitedCodec>,
        framed_read: FramedRead<ReadHalf<Io>, LengthDelimitedCodec>,
        idle_timeout: Option<Duration>,
    ) -> Self {
        let idle_timeout = match idle_timeout {
            Some(duration) => match duration.is_zero() {
                true => None,
                false => Some(IdleTimeout::new(duration)),
            },
            None => None,
        };

        Self {
            framed_write,
            framed_read,
            idle_timeout,
            ftype: PhantomData,
        }
    }
}

impl<Io> Transport<Io, ()>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    cfg_rustls! {
        /// Perform TLS negotiation with `tokio-rustls`
        pub async fn connect_tls_with_rustls(
            mut stream: Io,
            domain: &str,
            connector: &tokio_rustls::TlsConnector,
            alt_tls: bool,
        ) -> Result<tokio_rustls::client::TlsStream<Io>, NegotiationError> {
            use librustls::pki_types::ServerName;

            if !alt_tls {
                send_tls_proto_header(&mut stream).await?;
                let incoming_header = recv_tls_proto_header(&mut stream).await?;

                if !incoming_header.is_tls() {
                    return Err(NegotiationError::ProtocolHeaderMismatch(
                        incoming_header.into(),
                    ));
                }
            }

            // TLS negotiation
            let domain = ServerName::try_from(domain).map_err(|_| NegotiationError::InvalidDomain)?.to_owned();
            let tls = connector.connect(domain, stream).await?;
            Ok(tls)
        }
    }

    cfg_not_wasm32! {
        cfg_native_tls! {
            /// Perform TLS negotiation with `tokio-native-tls`
            pub async fn connect_tls_with_native_tls(
                mut stream: Io,
                domain: &str,
                connector: &tokio_native_tls::TlsConnector,
                alt_tls: bool,
            ) -> Result<tokio_native_tls::TlsStream<Io>, NegotiationError> {
                if !alt_tls {
                    send_tls_proto_header(&mut stream).await?;
                    let incoming_header = recv_tls_proto_header(&mut stream).await?;

                    if !incoming_header.is_tls() {
                        return Err(NegotiationError::ProtocolHeaderMismatch(
                            incoming_header.into(),
                        ));
                    }
                }

                connector.connect(domain, stream).await.map_err(|e| {
                    NegotiationError::Io(io::Error::new(io::ErrorKind::Other, format!("{:?}", e)))
                })
            }
        }
    }
}

impl<Io> Transport<Io, sasl::Frame>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    /// Performs SASL header negotiation
    ///
    /// This is separate from negotiate_amqp_header because SASL header exchange
    /// doesn't modify the connection state
    pub async fn negotiate_sasl_header(
        // mut framed: Framed<Io, ProtocolHeaderCodec>,
        mut framed_write: FramedWrite<WriteHalf<Io>, ProtocolHeaderCodec>,
        mut framed_read: FramedRead<ReadHalf<Io>, ProtocolHeaderCodec>,
    ) -> Result<Self, NegotiationError> {
        #[cfg(feature = "tracing")]
        let span = tracing::span!(tracing::Level::TRACE, "SEND");
        let proto_header = ProtocolHeader::sasl();
        #[cfg(feature = "tracing")]
        tracing::event!(parent: &span, tracing::Level::TRACE, ?proto_header);
        #[cfg(feature = "log")]
        log::trace!("SEND proto_header = {:?}", proto_header);
        framed_write.send(proto_header).await?;

        #[cfg(feature = "tracing")]
        let span = tracing::span!(tracing::Level::TRACE, "RECV");
        let incoming_header = framed_read.next().await.ok_or_else(|| {
            NegotiationError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Waiting for SASL header exchange",
            ))
        })??;
        #[cfg(feature = "tracing")]
        tracing::event!(parent: &span, tracing::Level::TRACE, ?incoming_header);
        #[cfg(feature = "log")]
        log::trace!("RECV incoming_header = {:?}", incoming_header);

        if !incoming_header.is_sasl()
            || incoming_header.major != MAJOR
            || incoming_header.minor != MINOR
            || incoming_header.revision != REVISION
        {
            return Err(NegotiationError::ProtocolHeaderMismatch(
                incoming_header.into(),
            ));
        }

        let encoder = length_delimited_encoder(MIN_MAX_FRAME_SIZE);
        let framed_write = framed_write.map_encoder(|_| encoder);
        let decoder = length_delimited_decoder(MIN_MAX_FRAME_SIZE);
        let framed_read = framed_read.map_decoder(|_| decoder);
        let transport = Self::bind_to_framed_codec(framed_write, framed_read, None);

        Ok(transport)
    }
}

impl<Io> Transport<Io, amqp::Frame>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    /// Performs AMQP negotiation
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    pub async fn negotiate_amqp_header(
        mut framed_write: FramedWrite<WriteHalf<Io>, ProtocolHeaderCodec>,
        mut framed_read: FramedRead<ReadHalf<Io>, ProtocolHeaderCodec>,
        local_state: &mut ConnectionState,
        idle_timeout: Option<Duration>,
    ) -> Result<Self, NegotiationError> {
        let proto_header = ProtocolHeader::amqp();
        send_amqp_proto_header(&mut framed_write, local_state, proto_header.clone()).await?;
        let _ = recv_amqp_proto_header(&mut framed_read, local_state, proto_header).await?;

        let encoder = length_delimited_encoder(MIN_MAX_FRAME_SIZE);
        let framed_write = framed_write.map_encoder(|_| encoder);
        let decoder = length_delimited_decoder(MIN_MAX_FRAME_SIZE);
        let framed_read = framed_read.map_decoder(|_| decoder);
        let transport = Transport::bind_to_framed_codec(framed_write, framed_read, idle_timeout);

        Ok(transport)
    }

    /// Change the max_frame_size for the transport length delimited encoder
    pub fn set_decoder_max_frame_size(&mut self, max_frame_size: usize) -> &mut Self {
        let max_frame_size = std::cmp::max(MIN_MAX_FRAME_SIZE, max_frame_size);
        self.framed_read
            .decoder_mut()
            .set_max_frame_length(max_frame_size);
        self
    }

    /// Get the max frame size of the encoder
    pub fn encoder_max_frame_size(&self) -> usize {
        self.framed_write.encoder().max_frame_length()
    }

    /// Change the max_frame_size for the transport length delimited decoder
    pub fn set_encoder_max_frame_size(&mut self, max_frame_size: usize) -> &mut Self {
        let max_frame_size = std::cmp::max(MIN_MAX_FRAME_SIZE, max_frame_size);
        self.framed_write
            .encoder_mut()
            .set_max_frame_length(max_frame_size - 4);
        self
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

/// Creates a LengthDelimitedCodec that can handle the AMQP and SASL frames
fn length_delimited_encoder(max_frame_size: usize) -> LengthDelimitedCodec {
    LengthDelimitedCodec::builder()
        .big_endian()
        .length_field_length(4)
        // Prior to any explicit negotiation,
        // the maximum frame size is 512 (MIN-MAX-FRAME-SIZE)
        .max_frame_length(max_frame_size - 4) // change max frame size later in negotiation
        .length_adjustment(-4)
        .new_codec()
}

fn length_delimited_decoder(max_frame_size: usize) -> LengthDelimitedCodec {
    LengthDelimitedCodec::builder()
        .big_endian()
        .length_field_length(4)
        .max_frame_length(max_frame_size)
        .length_adjustment(-4)
        .new_codec()
}

#[cfg_attr(feature = "tracing", tracing::instrument(name = "SEND", skip_all))]
pub(crate) async fn send_amqp_proto_header<W>(
    framed_write: &mut FramedWrite<W, ProtocolHeaderCodec>,
    local_state: &mut ConnectionState,
    proto_header: ProtocolHeader,
) -> Result<(), NegotiationError>
where
    W: AsyncWrite + Unpin,
{
    #[cfg(feature = "tracing")]
    tracing::trace!(?proto_header);
    #[cfg(feature = "log")]
    log::trace!("SEND proto_header = {:?}", proto_header);
    match local_state {
        ConnectionState::Start => {
            framed_write.send(proto_header).await?;
            *local_state = ConnectionState::HeaderSent;
        }
        ConnectionState::HeaderReceived => {
            framed_write.send(proto_header).await?;
            *local_state = ConnectionState::HeaderExchange;
        }
        _ => return Err(NegotiationError::IllegalState),
    }
    Ok(())
}

#[cfg_attr(feature = "tracing", tracing::instrument(name = "RECV", skip_all))]
async fn recv_amqp_proto_header<R>(
    framed_read: &mut FramedRead<R, ProtocolHeaderCodec>,
    local_state: &mut ConnectionState,
    proto_header: ProtocolHeader,
) -> Result<ProtocolHeader, NegotiationError>
where
    R: AsyncRead + Unpin,
{
    // wait for incoming header
    let proto_header = match local_state {
        ConnectionState::Start => {
            let incoming_header =
                read_and_compare_amqp_proto_header(framed_read, local_state, &proto_header).await?;
            *local_state = ConnectionState::HeaderReceived;
            incoming_header
        }
        ConnectionState::HeaderSent => {
            let incoming_header =
                read_and_compare_amqp_proto_header(framed_read, local_state, &proto_header).await?;
            *local_state = ConnectionState::HeaderExchange;
            incoming_header
        }
        _ => return Err(NegotiationError::IllegalState),
    };
    #[cfg(feature = "tracing")]
    tracing::trace!(?proto_header);
    #[cfg(feature = "log")]
    log::trace!("RECV proto_header = {:?}", proto_header);

    Ok(proto_header)
}

#[allow(unused)]
#[cfg(any(feature = "rustls", feature = "native-tls"))]
#[cfg_attr(feature = "tracing", tracing::instrument(name = "SEND", skip_all))]
async fn send_tls_proto_header<Io>(stream: &mut Io) -> Result<(), io::Error>
where
    Io: AsyncWrite + Unpin,
{
    use tokio::io::AsyncWriteExt;

    let proto_header = ProtocolHeader::tls();
    let buf: [u8; 8] = proto_header.into();
    stream.write_all(&buf).await
}

#[allow(unused)]
#[cfg(any(feature = "rustls", feature = "native-tls"))]
#[cfg_attr(feature = "tracing", tracing::instrument(name = "RECV", skip_all))]
pub(crate) async fn recv_tls_proto_header<Io>(
    stream: &mut Io,
) -> Result<ProtocolHeader, NegotiationError>
where
    Io: AsyncRead + Unpin,
{
    // use std::convert::TryFrom;
    use tokio::io::AsyncReadExt;

    let mut buf = [0u8; 8];
    stream.read_exact(&mut buf).await?;
    std::convert::TryFrom::try_from(buf).map_err(|buf| {
        NegotiationError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Invalid protocol header {:?}", buf),
        ))
    })
}

async fn read_and_compare_amqp_proto_header<R>(
    framed_read: &mut FramedRead<R, ProtocolHeaderCodec>,
    local_state: &mut ConnectionState,
    proto_header: &ProtocolHeader,
) -> Result<ProtocolHeader, NegotiationError>
where
    R: AsyncRead + Unpin,
{
    // check header
    let incoming_header = framed_read.next().await.ok_or_else(|| {
        NegotiationError::Io(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Waiting for header exchange",
        ))
    })??;
    if incoming_header != *proto_header {
        *local_state = ConnectionState::End;
        return Err(NegotiationError::NotImplemented(Some(format!(
            "Expecting {:?}, found {:?}",
            proto_header, incoming_header
        ))));
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
        this.framed_write
            .poll_ready(cx) // Result<_, std::io::Error>
            .map_err(Into::into)
    }

    fn start_send(
        mut self: std::pin::Pin<&mut Self>,
        item: amqp::Frame,
    ) -> Result<(), Self::Error> {
        use std::pin::Pin;

        let mut bytesmut = BytesMut::new();
        let max_frame_size = self.framed_write.encoder().max_frame_length();
        let mut encoder = amqp::FrameEncoder::new(max_frame_size);
        encoder.encode(item, &mut bytesmut)?;

        while bytesmut.len() > max_frame_size {
            let partial = bytesmut.split_to(max_frame_size);
            let writer = Pin::new(&mut self.framed_write);
            writer.start_send(partial.freeze())?;
        }

        let writer = Pin::new(&mut self.framed_write);
        writer
            .start_send(bytesmut.freeze()) // Result<_, std::io::Error>
            .map_err(Into::into)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed_write
            .poll_flush(cx) // Result<_, std::io::Error>
            .map_err(Into::into)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed_write
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
        match this.framed_read.poll_next(cx) {
            Poll::Ready(next) => {
                if let Some(mut delay) = this.idle_timeout.as_pin_mut() {
                    delay.reset();
                }

                match next {
                    Some(item) => {
                        let mut src = match item {
                            Ok(b) => b,
                            Err(err) => return Poll::Ready(Some(Err(err.into()))),
                        };
                        // tracing::debug!("raw bytes {:#x?}", &src[..]);
                        let mut decoder = amqp::FrameDecoder {};
                        Poll::Ready(decoder.decode(&mut src).map_err(Into::into).transpose())
                    }
                    None => Poll::Ready(None),
                }
            }
            Poll::Pending => {
                // check if idle timeout has exceeded
                if let Some(delay) = this.idle_timeout.as_pin_mut() {
                    match delay.poll(cx) {
                        Poll::Ready(_elapsed) => return Poll::Ready(Some(Err(Error::IdleTimeoutElapsed))),
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
        this.framed_write.poll_ready(cx).map_err(Into::into)
    }

    // #[instrument(skip_all)]
    fn start_send(self: std::pin::Pin<&mut Self>, item: sasl::Frame) -> Result<(), Self::Error> {
        // (frame=?item);

        // Needs to know the length, and thus cannot write directly to the IO
        let mut bytesmut = BytesMut::new();
        let mut encoder = sasl::FrameCodec {};
        encoder.encode(item, &mut bytesmut)?;

        let this = self.project();
        this.framed_write
            .start_send(bytesmut.freeze())
            .map_err(Into::into)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed_write.poll_flush(cx).map_err(Into::into)
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.framed_write.poll_close(cx).map_err(Into::into)
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

        match this.framed_read.poll_next(cx) {
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
    use bytes::{Bytes, BytesMut};
    use fe2o3_amqp_types::{performatives::Open, states::ConnectionState};
    use futures_util::{SinkExt, StreamExt};
    use tokio_test::io::Builder;
    use tokio_util::codec::{Encoder, FramedRead, FramedWrite, LengthDelimitedCodec};

    use crate::frames::amqp::FrameEncoder;

    use super::{
        amqp::{Frame, FrameBody},
        protocol_header::ProtocolHeaderCodec,
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

        let payload = Bytes::from(vec![b'a'; 512]);
        framed.send(payload).await.unwrap();

        let mut header = [0u8; 4];
        header.copy_from_slice(&writer[..4]);

        // test read
        let reader = &writer[..];
        let mut framed = LengthDelimitedCodec::builder()
            .big_endian()
            .length_field_length(4)
            .max_frame_length(512 + 4)
            .length_adjustment(-4)
            .new_read(reader);
        let _outcome = framed.next().await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_length_delimited_codec_on_empty_frame() {
        // test write
        let mut writer = vec![];
        let mut framed = LengthDelimitedCodec::builder()
            .big_endian()
            .length_field_length(4)
            .max_frame_length(512) // change max frame size later in negotiation
            .length_adjustment(-4)
            .new_write(&mut writer);

        let frame = Frame::empty();
        let mut encoder = FrameEncoder::new(512);
        let mut buf = BytesMut::new();
        encoder.encode(frame, &mut buf).unwrap();
        assert_eq!(&buf[..], &[0x2u8, 0x0, 0x0, 0x0]);

        framed.send(buf.freeze()).await.unwrap();
        assert_eq!(&writer[..], &[0x0u8, 0x0, 0x0, 0x8, 0x2, 0x0, 0x0, 0x0]);
    }

    #[tokio::test]
    async fn test_header_exchange() {
        let mock = Builder::new()
            .write(b"AMQP")
            .write(&[0, 1, 0, 0])
            .read(b"AMQP")
            .read(&[0, 1, 0, 0])
            .build();

        let (reader, writer) = tokio::io::split(mock);
        let framed_read = FramedRead::new(reader, ProtocolHeaderCodec::new());
        let framed_write = FramedWrite::new(writer, ProtocolHeaderCodec::new());

        let mut local_state = ConnectionState::Start;
        Transport::negotiate_amqp_header(framed_write, framed_read, &mut local_state, None)
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

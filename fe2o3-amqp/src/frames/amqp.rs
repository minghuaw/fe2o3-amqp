//! AMQP frame type and corresponding encoder and decoder

use bytes::{Buf, BufMut, BytesMut};
use fe2o3_amqp_types::performatives::{
    Attach, Begin, Close, Detach, Disposition, End, Flow, Open, Performative, Transfer,
};
use serde::{ser::Serialize, Deserialize};
use serde_amqp::{de::Deserializer, read::IoReader};
use tokio_util::codec::{Decoder, Encoder};

use crate::Payload;

use super::{Error, FRAME_TYPE_AMQP};

/// AMQP frame
#[derive(Debug)]
pub struct Frame {
    /// AMQP frame channel
    pub channel: u16,

    /// AMQP frame body
    pub body: FrameBody,
}

impl Frame {
    /// Creates a new AMQP frame
    pub fn new(channel: impl Into<u16>, body: FrameBody) -> Self {
        Self {
            channel: channel.into(),
            body,
        }
    }

    /// Get the channel of the frame
    pub fn channel(&self) -> u16 {
        self.channel
    }

    /// Get the body of the frame
    pub fn body(&self) -> &FrameBody {
        &self.body
    }

    /// Consume the frame to get the frame body
    pub fn into_body(self) -> FrameBody {
        self.body
    }

    /// Creates an emtpy frame. The empty frame is only used to reset
    /// the remote idle timeout
    pub fn empty() -> Self {
        Self {
            channel: 0,
            body: FrameBody::Empty,
        }
    }
}

/// Encoder and Decoder of the AMQP frames
pub struct FrameCodec {}

impl Encoder<Frame> for FrameCodec {
    type Error = Error;

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // AMQP frame ignores extended header, thus doff should always be 2
        dst.put_u8(2); // doff
        dst.put_u8(FRAME_TYPE_AMQP); // frame type
        dst.put_u16(item.channel);

        // encode frame body
        let mut encoder = FrameBodyCodec {};
        encoder.encode(item.body, dst)
    }
}

impl Decoder for FrameCodec {
    type Item = Frame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // use fe2o3_amqp_types::definitions::{AmqpError, ConnectionError};

        let doff = src.get_u8();
        let ftype = src.get_u8();
        let channel = src.get_u16();

        // check type byte
        if ftype != FRAME_TYPE_AMQP {
            return Err(Error::NotImplemented);
        }

        match doff {
            2 => {}
            _ => return Err(Error::NotImplemented),
        }

        // decode body
        let mut decoder = FrameBodyCodec {};
        let body = match decoder.decode(src)? {
            Some(b) => b,
            None => return Ok(None),
        };
        Ok(Some(Frame { channel, body }))
    }
}

/// AMQP frame body
#[derive(Debug)]
pub enum FrameBody {
    // Frames handled by Link
    
    /// Attach performative
    Attach(Attach),
    
    /// Flow performative
    Flow(Flow),

    /// Transfer performative and payload
    Transfer {
        /// Transfer performative
        performative: Transfer,

        /// Binary payload
        payload: Payload, // The payload should have ownership passed around not shared
    },

    /// Disposition performative
    Disposition(Disposition),

    /// Detach performative
    Detach(Detach),

    // Frames handled by Session

    /// Begin performative
    Begin(Begin),

    /// End performative
    End(End),

    // Frames handled by Connection

    /// Open performative
    Open(Open),

    /// Close performative
    Close(Close),

    /// An empty frame used only for resetting idle timeout
    Empty,
}

/// Encoder and Decoder for AMQP frame body
pub struct FrameBodyCodec {}

impl Encoder<FrameBody> for FrameBodyCodec {
    type Error = Error;

    fn encode(&mut self, item: FrameBody, dst: &mut BytesMut) -> Result<(), Self::Error> {
        use serde_amqp::ser::Serializer;

        let mut serializer = Serializer::from(dst.writer());
        match item {
            FrameBody::Open(performative) => performative.serialize(&mut serializer),
            FrameBody::Begin(performative) => performative.serialize(&mut serializer),
            FrameBody::Attach(performative) => performative.serialize(&mut serializer),
            FrameBody::Flow(performative) => performative.serialize(&mut serializer),
            FrameBody::Transfer {
                performative,
                payload,
            } => {
                performative.serialize(&mut serializer)?;
                dst.put(payload);
                Ok(())
            }
            FrameBody::Disposition(performative) => performative.serialize(&mut serializer),
            FrameBody::Detach(performative) => performative.serialize(&mut serializer),
            FrameBody::End(performative) => performative.serialize(&mut serializer),
            FrameBody::Close(performative) => performative.serialize(&mut serializer),
            FrameBody::Empty => Ok(()),
        }
        .map_err(Into::into)
    }
}

impl Decoder for FrameBodyCodec {
    type Item = FrameBody;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() == 0 {
            return Ok(Some(FrameBody::Empty));
        }
        let reader = IoReader::new(src.reader());
        let mut deserializer = Deserializer::new(reader);
        let performative: Performative = Deserialize::deserialize(&mut deserializer)?;

        let frame_body = match performative {
            Performative::Open(performative) => FrameBody::Open(performative),
            Performative::Begin(performative) => FrameBody::Begin(performative),
            Performative::Attach(performative) => FrameBody::Attach(performative),
            Performative::Transfer(performative) => {
                let payload = src.split().into();
                FrameBody::Transfer {
                    performative,
                    payload,
                }
            }
            Performative::Flow(performative) => FrameBody::Flow(performative),
            Performative::Disposition(performative) => FrameBody::Disposition(performative),
            Performative::Detach(performative) => FrameBody::Detach(performative),
            Performative::End(performative) => FrameBody::End(performative),
            Performative::Close(performative) => FrameBody::Close(performative),
        };

        Ok(Some(frame_body))
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use fe2o3_amqp_types::performatives::Open;
    use tokio_util::codec::{Decoder, Encoder};

    use super::{Frame, FrameBody, FrameBodyCodec, FrameCodec};

    #[test]
    fn test_encoding_frame_body() {
        let open = Open {
            container_id: "1234".into(),
            hostname: Some("127.0.0.1".into()),
            max_frame_size: 100.into(),
            channel_max: 9.into(),
            idle_time_out: Some(10),
            outgoing_locales: None,
            incoming_locales: None,
            offered_capabilities: None,
            desired_capabilities: None,
            properties: None,
        };

        let body = FrameBody::Open(open);

        let mut encoder = FrameBodyCodec {};
        let mut dst = BytesMut::new();
        encoder.encode(body, &mut dst).unwrap();
        println!("{:?}", dst);
    }

    #[test]
    fn test_encoding_empty_frame() {
        let empty = Frame::empty();
        let mut encoder = FrameCodec {};
        let mut dst = BytesMut::new();
        encoder.encode(empty, &mut dst).unwrap();
        println!("{:x?}", dst);
    }

    #[test]
    fn test_decode_empty_frame() {
        let mut decoder = FrameCodec {};
        let mut src = BytesMut::from(&[0x02, 0x00, 0x00, 0x00][..]);
        let frame = decoder.decode(&mut src).unwrap();
        println!("{:?}", frame);
    }
}

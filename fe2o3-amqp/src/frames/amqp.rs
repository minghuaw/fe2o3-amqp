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

/// Encoder of the AMQP frames
#[derive(Debug)]
pub struct FrameEncoder {
    /// Max frame size for the transport
    max_frame_body_size: usize,
}

fn write_header(dst: &mut BytesMut, channel: u16) {
    // AMQP frame ignores extended header, thus doff should always be 2
    dst.put_u8(2); // doff
    dst.put_u8(FRAME_TYPE_AMQP); // frame type
    dst.put_u16(channel);
}

impl FrameEncoder {
    pub(crate) fn new(max_frame_size: usize) -> Self {
        Self {
            max_frame_body_size: max_frame_size - 4,
        }
    }

    fn encode_transfer(
        &self,
        dst: &mut BytesMut,
        channel: u16,
        mut transfer: Transfer,
        mut payload: Payload,
    ) -> Result<(), serde_amqp::Error> {
        use serde_amqp::ser::Serializer;

        // First test the size
        let mut buf = BytesMut::new();
        let writer = (&mut buf).writer();
        let mut buf_serializer = Serializer::from(writer);
        transfer.serialize(&mut buf_serializer)?;
        let remaining_bytes = buf.len() + payload.len();
        let more = remaining_bytes > self.max_frame_body_size;
        if more {
            let orig_more = transfer.more; // If the transfer is pre-split at link
            transfer.more = true;
            buf.clear();
            let writer = (&mut buf).writer();
            let mut serializer = Serializer::from(writer);
            transfer.serialize(&mut serializer)?;
            let split_index = self.max_frame_body_size - buf.len();

            // Send first frame
            let partial = payload.split_to(split_index);
            write_header(dst, channel);
            dst.put(&buf[..]);
            dst.put(partial);

            // Send middle frames
            transfer.delivery_id = None;
            transfer.delivery_tag = None;
            transfer.message_format = None;
            transfer.settled = None;
            transfer.rcv_settle_mode = None;
            buf.clear();
            let writer = (&mut buf).writer();
            let mut serializer = Serializer::from(writer);
            transfer.serialize(&mut serializer)?;

            let mut remaining_bytes = buf.len() + payload.len();
            let split_index = self.max_frame_body_size - buf.len();

            while remaining_bytes > self.max_frame_body_size {
                // The transfer performative can be kept the same for the first n-1 frames
                let partial = payload.split_to(split_index);
                write_header(dst, channel);
                dst.put(&buf[..]);
                dst.put(partial);

                remaining_bytes = buf.len() + payload.len();
            }

            // Send last frame
            transfer.more = orig_more;
            buf.clear();
            let writer = (&mut buf).writer();
            let mut serializer = Serializer::from(writer);
            transfer.serialize(&mut serializer)?;

            write_header(dst, channel);
            dst.put(buf);
            dst.put(payload);
        } else {
            write_header(dst, channel);
            dst.put(buf);
            dst.put(payload);
        }

        Ok(())
    }
}

impl Encoder<Frame> for FrameEncoder {
    type Error = Error;

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        use serde_amqp::ser::Serializer;

        match item.body {
            FrameBody::Open(performative) => {
                write_header(dst, item.channel);
                let mut serializer = Serializer::from(dst.writer());
                performative.serialize(&mut serializer)
            }
            FrameBody::Begin(performative) => {
                write_header(dst, item.channel);
                let mut serializer = Serializer::from(dst.writer());
                performative.serialize(&mut serializer)
            }
            FrameBody::Attach(performative) => {
                write_header(dst, item.channel);
                let mut serializer = Serializer::from(dst.writer());
                performative.serialize(&mut serializer)
            }
            FrameBody::Flow(performative) => {
                write_header(dst, item.channel);
                let mut serializer = Serializer::from(dst.writer());
                performative.serialize(&mut serializer)
            }
            FrameBody::Transfer {
                performative,
                payload,
            } => self.encode_transfer(dst, item.channel, performative, payload),
            FrameBody::Disposition(performative) => {
                write_header(dst, item.channel);
                let mut serializer = Serializer::from(dst.writer());
                performative.serialize(&mut serializer)
            }
            FrameBody::Detach(performative) => {
                write_header(dst, item.channel);
                let mut serializer = Serializer::from(dst.writer());
                performative.serialize(&mut serializer)
            }
            FrameBody::End(performative) => {
                write_header(dst, item.channel);
                let mut serializer = Serializer::from(dst.writer());
                performative.serialize(&mut serializer)
            }
            FrameBody::Close(performative) => {
                write_header(dst, item.channel);
                let mut serializer = Serializer::from(dst.writer());
                performative.serialize(&mut serializer)
            }
            FrameBody::Empty => {
                write_header(dst, item.channel);
                Ok(())
            }
        }
        .map_err(Into::into)
    }
}

/// Decoder of the AMQP frames
#[derive(Debug)]
pub struct FrameDecoder {}

impl Decoder for FrameDecoder {
    type Item = Frame;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
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

        let body = if src.is_empty() {
            FrameBody::Empty
        } else {
            let reader = IoReader::new(src.reader());
            let mut deserializer = Deserializer::new(reader);
            let performative: Performative = Deserialize::deserialize(&mut deserializer)?;

            match performative {
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
            }
        };

        Ok(Some(Frame { channel, body }))
    }
}

/// AMQP frame body
// #[derive(Debug)]
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

impl std::fmt::Debug for FrameBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attach(arg0) => f.debug_tuple("Attach").field(arg0).finish(),
            Self::Flow(arg0) => f.debug_tuple("Flow").field(arg0).finish(),
            Self::Transfer {
                performative,
                payload,
            } => f
                .debug_struct("Transfer")
                .field("performative", performative)
                .field("payload.len", &payload.len())
                .finish(),
            Self::Disposition(arg0) => f.debug_tuple("Disposition").field(arg0).finish(),
            Self::Detach(arg0) => f.debug_tuple("Detach").field(arg0).finish(),
            Self::Begin(arg0) => f.debug_tuple("Begin").field(arg0).finish(),
            Self::End(arg0) => f.debug_tuple("End").field(arg0).finish(),
            Self::Open(arg0) => f.debug_tuple("Open").field(arg0).finish(),
            Self::Close(arg0) => f.debug_tuple("Close").field(arg0).finish(),
            Self::Empty => write!(f, "Empty"),
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use tokio_util::codec::{Decoder, Encoder};

    use crate::frames::amqp::{FrameDecoder, FrameEncoder};

    use super::Frame;

    #[test]
    fn test_encoding_empty_frame() {
        let empty = Frame::empty();
        let mut encoder = FrameEncoder::new(512);
        let mut dst = BytesMut::new();
        encoder.encode(empty, &mut dst).unwrap();
        println!("{:x?}", dst);
    }

    #[test]
    fn test_decode_empty_frame() {
        let mut decoder = FrameDecoder {};
        let mut src = BytesMut::from(&[0x02, 0x00, 0x00, 0x00][..]);
        let frame = decoder.decode(&mut src).unwrap();
        println!("{:?}", frame);
    }
}

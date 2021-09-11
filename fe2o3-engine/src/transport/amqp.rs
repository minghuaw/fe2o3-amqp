use bytes::{Buf, BufMut, BytesMut};
use fe2o3_amqp::{de::Deserializer, read::{IoReader}};
use fe2o3_types::performatives::{Attach, Begin, Close, Detach, Disposition, End, Flow, Open, Performative, Transfer};
use serde::{Deserialize, ser::Serialize};
use tokio_util::codec::{Decoder, Encoder};

use crate::error::EngineError;

use super::{FRAME_TYPE_AMQP};

pub struct Frame {
    pub channel: u16,
    pub body: FrameBody
}

impl Frame {
    pub fn new(channel: impl Into<u16>, body: FrameBody) -> Self {
        Self {
            channel: channel.into(),
            body
        }
    }

    pub fn channel(&self) -> u16 {
        self.channel
    }

    pub fn body(&self) -> &FrameBody {
        &self.body
    }

    pub fn into_body(self) -> FrameBody {
        self.body
    }
}

pub struct FrameCodec {}

impl Encoder<Frame> for FrameCodec {
    type Error = EngineError;

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // AMQP frame ignores extended header, thus doff should always be 2
        dst.put_u8(2); // doff
        dst.put_u8(FRAME_TYPE_AMQP); // frame type
        dst.put_u16(item.channel);

        // encode frame body
        let mut encoder = FrameBodyCodec{};
        encoder.encode(item.body, dst)
    }
}

impl Decoder for FrameCodec {
    type Item = Frame;
    type Error = EngineError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let doff = src.get_u8();
        let ftype = src.get_u8();
        let channel = src.get_u16();

        // check type byte
        if ftype != FRAME_TYPE_AMQP {
            return Err(EngineError::Message("Only AMQP frame is implemented for now"))
        }

        match doff {
            2 => {},
            // 0..=1 => return Err(EngineError::MalformedFrame),
            _ => return Err(EngineError::MalformedFrame),
        }

        // decode body
        let mut decoder = FrameBodyCodec {};
        let body = match decoder.decode(src)? {
            Some(b) => b,
            None => return Ok(None)
        };
        Ok(Some(
            Frame{channel, body}
        ))
    }
}


pub enum FrameBody {
    Open{
        performative: Open
    },
    Begin{
        performative: Begin
    },
    Attach{
        performative: Attach
    },
    Flow{
        performative: Flow
    },
    Transfer{
        performative: Transfer,
        payload: Option<BytesMut>,
    },
    Disposition {
        performative: Disposition
    },
    Detach {
        performative: Detach
    },
    End {
        performative: End
    },
    Close {
        performative: Close
    }
}

impl FrameBody {
    /// The payload will be ignored unless the performative is Transfer
    pub fn from_parts(performative: Performative, payload: Option<BytesMut>) -> Self {
        match performative {
            Performative::Open(performative) => FrameBody::Open{performative},
            Performative::Begin(performative) => FrameBody::Begin{performative},
            Performative::Attach(performative) => FrameBody::Attach{performative},
            Performative::Flow(performative) => FrameBody::Flow{performative},
            Performative::Transfer(performative) => FrameBody::Transfer{performative, payload},
            Performative::Disposition(performative) => FrameBody::Disposition{performative},
            Performative::Detach(performative) => FrameBody::Detach{performative},
            Performative::End(performative) => FrameBody::End{performative},
            Performative::Close(performative) => FrameBody::Close{performative},
        }
    }
}

pub struct FrameBodyCodec {}

impl Encoder<FrameBody> for FrameBodyCodec {
    type Error = EngineError;

    fn encode(&mut self, item: FrameBody, dst: &mut BytesMut) -> Result<(), Self::Error> {
        use fe2o3_amqp::ser::Serializer;

        let mut serializer = Serializer::from(dst.as_mut());
        match item {
            FrameBody::Open{performative} => performative.serialize(&mut serializer),
            FrameBody::Begin{performative} => performative.serialize(&mut serializer),
            FrameBody::Attach{performative} => performative.serialize(&mut serializer),
            FrameBody::Flow{performative} => performative.serialize(&mut serializer),
            FrameBody::Transfer{performative, payload} => {
                performative.serialize(&mut serializer)?;
                if let Some(payload) = payload {
                    dst.put(payload);
                }
                Ok(())
            },
            FrameBody::Disposition { performative } => performative.serialize(&mut serializer),
            FrameBody::Detach{performative} => performative.serialize(&mut serializer),
            FrameBody::End{performative} => performative.serialize(&mut serializer),
            FrameBody::Close{performative} => performative.serialize(&mut serializer),

        }.map_err(Into::into)
    }
}

impl Decoder for FrameBodyCodec {
    type Item = FrameBody;
    type Error = EngineError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let reader = IoReader::new(src.reader());
        let mut deserializer = Deserializer::new(reader);
        let performative: Performative = Deserialize::deserialize(&mut deserializer)?;

        let frame_body = match performative {
            Performative::Transfer(performative) => {
                let payload = if src.has_remaining() {
                    Some(src.split())
                } else {
                    None
                };
                FrameBody::Transfer {performative, payload}
            },
            p @ _ => FrameBody::from_parts(p, None)
        };

        Ok(Some(frame_body))
    }
}

#[cfg(test)]
mod tests {

}
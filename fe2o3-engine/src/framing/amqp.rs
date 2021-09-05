use bytes::{Buf, BufMut, BytesMut};
use fe2o3_amqp::{de::Deserializer, read::{IoReader}};
use fe2o3_types::performatives::Performative;
use serde::{Deserialize, ser::Serialize};
use tokio_util::codec::{Decoder, Encoder};

use crate::error::EngineError;

use super::{FRAME_TYPE_AMQP};

pub struct AmqpFrame {
    channel: u16,
    ext_header: Option<BytesMut>,
    body: AmqpFrameBody,
}

impl AmqpFrame {
    pub fn new(channel: u16, ext_header: Option<BytesMut>, body: AmqpFrameBody) -> Self {
        Self { channel, ext_header, body }
    }

    pub fn channel(&self) -> &u16 {
        &self.channel
    }

    pub fn channel_mut(&mut self) -> &mut u16 {
        &mut self.channel
    }

    pub fn body(&self) -> &AmqpFrameBody {
        &self.body
    }

    pub fn body_mut(&mut self) -> &mut AmqpFrameBody {
        &mut self.body
    }
}

pub struct AmqpFrameBody {
    pub performative: Performative,
    pub payload: Option<BytesMut>,
}

impl AmqpFrameBody {
    pub fn new(performative: Performative, payload: Option<BytesMut>) -> Self {
        Self {
            performative,
            payload,
        }
    }

    pub fn performative(&self) -> &Performative {
        &self.performative
    }

    pub fn payload(&self) -> Option<&BytesMut> {
        (&self.payload).as_ref()
    }
}

pub struct AmqpFrameBodyEncoder {}

impl Encoder<AmqpFrameBody> for AmqpFrameBodyEncoder {
    type Error = EngineError;

    fn encode(&mut self, item: AmqpFrameBody, dst: &mut BytesMut) -> Result<(), Self::Error> {
        use fe2o3_amqp::ser::Serializer;

        // serialize performative
        let mut serializer = Serializer::from(dst.as_mut());
        Serialize::serialize(&item.performative, &mut serializer)?;

        // copy payload. FIXME: extend from payload
        if let Some(payload) = item.payload {
            dst.put(payload)
        }
        Ok(())
    }
}

pub struct AmqpFrameBodyDecoder {}

impl Decoder for AmqpFrameBodyDecoder {
    type Item = AmqpFrameBody;
    type Error = EngineError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let reader = IoReader::new(src.reader());
        let mut deserializer = Deserializer::new(reader);
        let performative: Performative = Deserialize::deserialize(&mut deserializer)?;

        let buf_mut = deserializer.reader_mut().get_mut().get_mut();
        let payload = if buf_mut.has_remaining()
        {
            Some(buf_mut.split())
        } else {
            None
        };

        Ok(Some(
            AmqpFrameBody::new(performative, payload)
        ))
    }
}

pub struct AmqpFrameEncoder {}

impl Encoder<AmqpFrame> for AmqpFrameEncoder {
    type Error = EngineError;

    // FIXME: doff needs to be calculated at runtime
    fn encode(&mut self, item: AmqpFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // encode header
        // let mut encoder = AmqpFrameHeaderEncoder {};
        // encoder.encode(item.header, dst)?;
        let doff = match &item.ext_header {
            Some(eh) => match eh.len() {
                0 => 2u8,
                l @ _ => (2 + l / 4) as u8
            },
            None => 2u8
        };

        dst.put_u8(doff);
        dst.put_u8(FRAME_TYPE_AMQP);
        dst.put_u16(item.channel);

        // encode extended header
        if let Some(ext_header) = item.ext_header {
            dst.put(ext_header)
        }

        // encode body
        let mut encoder = AmqpFrameBodyEncoder {};
        encoder.encode(item.body, dst)
    }
}

pub struct AmqpFrameDecoder {}

impl Decoder for AmqpFrameDecoder {
    type Item = AmqpFrame;
    type Error = EngineError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let doff = src.get_u8();
        let ftype = src.get_u8();
        let channel = src.get_u16();

        // check type byte
        if ftype != FRAME_TYPE_AMQP {
            return Err(EngineError::Message("Only AMQP frame is implemented for now"))
        }

        // decode extended header if there is any
        let ext_header = match doff {
            0..=1 => return Err(EngineError::MalformedFrame),
            2 => {
                None
            }
            v @ _ => {
                let len = (v as usize) * 4 - 8;
                Some(src.split_to(len))
            }
        };

        let mut body_decoder = AmqpFrameBodyDecoder {};
        let body = match body_decoder.decode(src)? {
            Some(b) => b,
            None => return Ok(None)
        };
        Ok(Some(
            AmqpFrame::new(channel, ext_header, body)
        ))
    }
}

#[cfg(test)]
mod tests {

}
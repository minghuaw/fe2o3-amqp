use bytes::{Buf, BufMut, BytesMut};
use fe2o3_amqp::{de::Deserializer, read::{IoReader}};
use fe2o3_types::performatives::Performative;
use serde::{Deserialize, ser::Serialize};
use tokio_util::codec::{Decoder, Encoder};

use crate::error::EngineError;

use super::{FRAME_TYPE_AMQP};

pub struct AmqpFrame {
    header: AmqpFrameHeader,
    ext_header: Option<BytesMut>,
    body: AmqpFrameBody,
}

impl AmqpFrame {
    pub fn new(header: AmqpFrameHeader, ext_header: Option<BytesMut>, body: AmqpFrameBody) -> Self {
        Self { header, ext_header, body }
    }

    pub fn header(&self) -> &AmqpFrameHeader {
        &self.header
    }

    pub fn header_mut(&mut self) -> &mut AmqpFrameHeader {
        &mut self.header
    }

    pub fn body(&self) -> &AmqpFrameBody {
        &self.body
    }

    pub fn body_mut(&mut self) -> &mut AmqpFrameBody {
        &mut self.body
    }
}

pub struct AmqpFrameHeader {
    pub doff: u8,
    pub channel: u16,
}

impl AmqpFrameHeader {
    pub fn new(doff: u8, channel: u16) -> Self {
        Self { doff, channel }
    }

    pub fn data_offset(&self) -> u8 {
        self.doff
    }

    pub fn channel(&self) -> u16 {
        self.channel
    }
}

pub struct AmqpFrameHeaderEncoder {}

impl Encoder<AmqpFrameHeader> for AmqpFrameHeaderEncoder {
    type Error = EngineError;

    // The 4 bytes frame size will be encoded with the LengthDelimitedCodec
    fn encode(&mut self, item: AmqpFrameHeader, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put_u8(item.doff);
        dst.put_u8(FRAME_TYPE_AMQP);
        dst.put_u16(item.channel);
        Ok(())
    }
}

pub struct AmqpFrameHeaderDecoder {}

impl Decoder for AmqpFrameHeaderDecoder {
    type Item = AmqpFrameHeader;
    type Error = EngineError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            // return Err(EngineError::ParseError(fe2o3_amqp::Error::InvalidLength))
            return Ok(None)
        }

        // read four bytes
        let doff = src.get_u8();
        let ftype = src.get_u8();
        let channel = src.get_u16();

        // check type byte
        if ftype != FRAME_TYPE_AMQP {
            return Err(EngineError::Message("Only AMQP frame is implemented for now"))
        }
        Ok(Some(
            AmqpFrameHeader::new(doff, channel)
        ))
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
        let mut encoder = AmqpFrameHeaderEncoder {};
        encoder.encode(item.header, dst)?;

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
        // decode AmqpFrameHeader
        let mut header_decoder = AmqpFrameHeaderDecoder {};
        let header = match header_decoder.decode(src)? {
            Some(h) => h,
            None => return Ok(None)
        };

        // decode extended header if there is any
        let ext_header = match header.data_offset() {
            0 => None,
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
            AmqpFrame::new(header, ext_header, body)
        ))
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use tokio_util::codec::Encoder;

    use super::{AmqpFrameHeader, AmqpFrameHeaderDecoder, AmqpFrameHeaderEncoder};

    #[test]
    fn test_encode_frame_header() {
        let mut dst = BytesMut::new();
        let mut encoder = AmqpFrameHeaderEncoder {};
    }
}
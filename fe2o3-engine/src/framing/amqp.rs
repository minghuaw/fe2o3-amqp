use bytes::{Buf, BufMut, BytesMut};
use fe2o3_amqp::{de::Deserializer, read::{IoReader}};
use fe2o3_types::performatives::Performative;
use serde::{Deserialize, ser::Serialize};
use tokio_util::codec::{Decoder, Encoder};

use crate::error::EngineError;

use super::{FRAME_TYPE_AMQP};

pub struct AmqpFrame {
    header: AmqpFrameHeader,
    body: AmqpFrameBody,
}

impl AmqpFrame {
    pub fn new(header: AmqpFrameHeader, body: AmqpFrameBody) -> Self {
        Self { header, body }
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
            dst.unsplit(payload)
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

    fn encode(&mut self, item: AmqpFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut encoder = AmqpFrameHeaderEncoder {};
        encoder.encode(item.header, dst)?;

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

        let mut body_decoder = AmqpFrameBodyDecoder {};
        let body = match body_decoder.decode(src)? {
            Some(b) => b,
            None => return Ok(None)
        };
        Ok(Some(
            AmqpFrame::new(header, body)
        ))
    }
}
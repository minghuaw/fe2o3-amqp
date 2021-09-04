use bytes::{BufMut, BytesMut};
use fe2o3_amqp::to_vec;
use fe2o3_types::performatives::Performative;
use tokio_util::codec::Encoder;
use serde::ser::Serialize;

use crate::error::EngineError;

use super::FRAME_TYPE_AMQP;

pub struct AmqpFrame<T> {
    header: AmqpFrameHeader,
    body: AmqpFrameBody<T>
}

impl<T> AmqpFrame<T> {
    pub fn new(header: AmqpFrameHeader, body: AmqpFrameBody<T>) -> Self {
        Self {
            header,
            body
        }
    }

    pub fn header(&self) -> &AmqpFrameHeader {
        &self.header
    }

    pub fn header_mut(&mut self) -> &mut AmqpFrameHeader {
        &mut self.header
    }

    pub fn body(&self) -> &AmqpFrameBody<T> {
        &self.body
    }

    pub fn body_mut(&mut self) -> &mut AmqpFrameBody<T> {
        &mut self.body
    }
}

pub struct AmqpFrameHeader {
    pub doff: u8,
    pub channel: u16
}

impl AmqpFrameHeader {
    pub fn new(doff: u8, channel: u16) -> Self {
        Self {
            doff,
            channel
        }
    }

    pub fn data_offset(&self) -> u8 {
        self.doff
    }

    pub fn channel(&self) -> u16 {
        self.channel
    }
}

pub struct AmqpFrameBody<T> {
    pub performative: Performative,
    pub payload: Option<T>,
}

impl<T> AmqpFrameBody<T> {
    pub fn new(performative: Performative, payload: Option<T>) -> Self {
        Self {
            performative,
            payload
        }
    }

    pub fn performative(&self) -> &Performative {
        &self.performative
    }

    pub fn payload(&self) -> Option<&T> {
        (&self.payload).as_ref()
    }
}

pub struct AmqpFrameHeaderEncoder { }

impl Encoder<AmqpFrameHeader> for AmqpFrameHeaderEncoder {
    type Error = EngineError;

    fn encode(&mut self, item: AmqpFrameHeader, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put_u8(item.doff);
        dst.put_u8(FRAME_TYPE_AMQP);
        dst.put_u16(item.channel);
        Ok(())
    }
}

pub struct AmqpFrameBodyEncoder { }

impl<T: Serialize> Encoder<AmqpFrameBody<T>> for AmqpFrameBodyEncoder {
    type Error = EngineError;

    fn encode(&mut self, item: AmqpFrameBody<T>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        use fe2o3_amqp::ser::Serializer;
        
        // serialize performative
        let mut serializer = Serializer::from(dst.as_mut());
        Serialize::serialize(&item.performative, &mut serializer)?;

        // serialize payload
        if let Some(payload) = item.payload {
            Serialize::serialize(&payload, &mut serializer)?;
        }
        Ok(())
    }
}

pub struct AmqpFrameEncoder { }

impl<T: Serialize> Encoder<AmqpFrame<T>> for AmqpFrameEncoder {
    type Error = EngineError;

    fn encode(&mut self, item: AmqpFrame<T>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut encoder = AmqpFrameHeaderEncoder { };
        encoder.encode(item.header, dst)?;

        let mut encoder = AmqpFrameBodyEncoder { };
        encoder.encode(item.body, dst)
    }
}
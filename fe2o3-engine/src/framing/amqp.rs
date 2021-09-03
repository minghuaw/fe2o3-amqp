use bytes::BytesMut;
use fe2o3_types::performatives::Performative;

pub struct AmqpFrame {
    header: AmqpFrameHeader,
    body: AmqpFrameBody
}

pub struct AmqpFrameHeader {
    doff: u8,
    channel: u16
}

pub struct AmqpFrameBody {
    performative: Performative,
    payload: BytesMut,
}
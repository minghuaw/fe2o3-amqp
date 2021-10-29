use bytes::BytesMut;
use fe2o3_amqp_types::performatives::{Attach, Detach, Disposition, Flow, Transfer};

pub enum LinkFrame {
    Attach(Attach),
    Flow(Flow),
    Transfer {
        performative: Transfer,
        payload: Option<BytesMut>,
    },
    Disposition(Disposition),
    Detach(Detach),
}
use fe2o3_amqp_types::{messaging::Message, primitives::Value};

pub trait IntoTunneled {
    fn into_tunneled(self) -> Message<Value>;
}
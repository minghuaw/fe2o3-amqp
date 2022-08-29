use std::num::NonZeroU16;

use fe2o3_amqp_types::primitives::SimpleValue;

/// HTTP status code
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct StatusCode(NonZeroU16);

impl TryFrom<SimpleValue> for StatusCode {
    type Error = SimpleValue;

    fn try_from(value: SimpleValue) -> Result<Self, Self::Error> {
        todo!()
    }
}

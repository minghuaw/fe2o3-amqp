use fe2o3_amqp_types::definitions::AmqpError;

/// TODO:
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("AMQP error {:?}, {:?}", .condition, .description)]
    AmqpError {
        condition: AmqpError,
        description: Option<String>,
    },
}

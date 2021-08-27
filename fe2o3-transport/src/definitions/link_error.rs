use fe2o3_amqp::macros::NonDescribed;

/// TODO: manually implement Serialize and Deserialize
#[derive(Debug, NonDescribed)]
pub enum LinkError {
    DetachForced,
    TransferLimitExceeded,
    MessageSizeExceeded,
    Redirect,
    Stolen,
}
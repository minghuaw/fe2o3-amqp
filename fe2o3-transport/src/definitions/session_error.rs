use fe2o3_amqp::macros::NonDescribed;

/// TODO: manually implement serialize and deserialize
#[derive(Debug, NonDescribed)]
pub enum SessionError {
    WindowViolation,
    ErrantLink,
    HandleInUse,
    UnattachedHandle
}
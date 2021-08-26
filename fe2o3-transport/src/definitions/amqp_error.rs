
/// TODO: manually implement serialize and deserialize
#[derive(Debug)]
pub enum AmqpError {
    InternalError,
    NotFound,
    UnauthorizedAccess,
    DecodeError,
    ResourceLimitExceeded,
    NotAllowed,
    InvalidField,
    NotImplemented,
    ResourceLocked,
    PreconditionFailed,
    ResourceDeleted,
    IllegalState,
    FrameSizeTooSmall
}
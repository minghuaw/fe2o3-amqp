
/// TODO: manually implement Serialize and Deserialize
#[derive(Debug)]
pub enum LinkError {
    DetachForced,
    TransferLimitExceeded,
    MessageSizeExceeded,
    Redirect,
    Stolen,
}
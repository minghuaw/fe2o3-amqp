
/// TODO: manually implement serialize and deserialize
#[derive(Debug)]
pub enum ConnectionError {
    ConnectionForced,
    FramingError,
    Redirect
}
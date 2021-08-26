
/// TODO: manually implement serialize and deserialize
#[derive(Debug)]
pub enum SessionError {
    WindowViolation,
    ErrantLink,
    HandleInUse,
    UnattachedHandle
}
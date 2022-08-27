use std::num::NonZeroU16;

/// HTTP status code
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct StatusCode(NonZeroU16);
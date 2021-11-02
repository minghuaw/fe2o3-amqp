mod frame;
pub use frame::*;
pub mod builder;
pub mod receiver;
pub mod sender;
pub mod receiver_link;
pub mod sender_link;

pub use receiver::Receiver;
pub use sender::Sender;


pub mod role {

    /// Type state for link::builder::Builder
    pub struct Sender {}

    /// Type state for link::builder::Builder
    pub struct Receiver {}
}

pub enum LinkState {
    /// The initial state after initialization
    Unattached,

    /// An attach frame has been sent
    AttachSent,

    /// An attach frame has been received
    AttachReceived,

    /// The link is attached
    Attached,

    /// A detach frame has been sent
    DetachSent,

    /// A detach frame has been received
    DetachReceived,

    /// The link is detached
    Detached,
}

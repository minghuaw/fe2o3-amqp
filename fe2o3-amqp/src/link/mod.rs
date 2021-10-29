pub mod frame;
pub mod sender;
pub mod receiver;

pub use sender::SenderLink;
pub use receiver::ReceiverLink;

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
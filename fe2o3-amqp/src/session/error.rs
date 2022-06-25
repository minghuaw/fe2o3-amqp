use std::io;

use fe2o3_amqp_types::definitions::{self, AmqpError, ErrorCondition, Handle, SessionError};
use tokio::task::JoinError;

use crate::connection::AllocSessionError;

#[cfg(feature = "transaction")]
use crate::link::AttachError;

/// Session errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Io Error. If includes failing to send to the underlying connection event loop
    #[error(transparent)]
    Io(#[from] io::Error),

    /// An attempt is trying to exceed the maximum number of allowed channel
    #[error("All channels have been allocated")]
    ChannelMaxReached,

    /// Error joining the event loop task. This could occur only when the user attempts
    /// to end the session
    #[error(transparent)]
    JoinError(#[from] JoinError),

    /// TODO: Fine grain control over particular errors
    ///
    /// A local error
    #[error("Local error {:?}", .0)]
    Local(definitions::Error),

    /// A peer that receives a handle outside the supported range MUST close the connection with the
    /// framing-error error-code
    #[error("A handle outside the supported range is received")]
    HandleMaxExceeded,

    /// The remote peer ended the session with the provided error
    #[error("Remote error {:?}", .0)]
    Remote(definitions::Error),

    /// TODO: hide this from public API
    /// Link handle error should be handled differently. Link handle is only local
    #[error("Local LinkRelay {:?} error {:?}", .handle, .error)]
    LinkHandleError {
        /// Handle of the link
        handle: Handle,
        /// Whether the link should close upon having an error
        closed: bool,
        /// Error
        error: definitions::Error,
    },

    /// A remotely initiated control link failed to attach
    ///
    /// TODO: Hide from public API?
    #[cfg(feature = "transaction")]
    #[error("Control link attach error {:?}", .0)]
    CoordinatorAttachError(AttachError),
}

impl Error {
    // pub(crate) fn decode_error() -> Self {
    //     Self::Local(definitions::Error::new(
    //         AmqpError::DecodeError,
    //         None,
    //         None
    //     ))
    // }

    pub(crate) fn amqp_error(
        condition: impl Into<AmqpError>,
        description: impl Into<Option<String>>,
    ) -> Self {
        Self::Local(definitions::Error {
            condition: ErrorCondition::AmqpError(condition.into()),
            description: description.into(),
            info: None,
        })
    }

    pub(crate) fn session_error(
        condition: impl Into<SessionError>,
        description: impl Into<Option<String>>,
    ) -> Self {
        Self::Local(definitions::Error {
            condition: ErrorCondition::SessionError(condition.into()),
            description: description.into(),
            info: None,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AllocLinkError {
    #[error("Illegal session state")]
    IllegalSessionState,

    #[error("Link name must be unique")]
    DuplicatedLinkName,
}

impl From<AllocSessionError> for Error {
    fn from(err: AllocSessionError) -> Self {
        match err {
            AllocSessionError::Io(e) => Self::Io(e),
            AllocSessionError::ChannelMaxReached => Self::ChannelMaxReached,
            AllocSessionError::IllegalState => {
                Self::Local(definitions::Error::new(AmqpError::IllegalState, None, None))
            }
        }
    }
}

impl From<AllocLinkError> for definitions::Error {
    fn from(err: AllocLinkError) -> Self {
        match err {
            AllocLinkError::IllegalSessionState => Self {
                condition: AmqpError::IllegalState.into(),
                description: Some("Illegal session state".to_string()),
                info: None,
            },
            AllocLinkError::DuplicatedLinkName => Self {
                condition: AmqpError::NotAllowed.into(),
                description: Some("Link name is duplicated".to_string()),
                info: None,
            },
        }
    }
}

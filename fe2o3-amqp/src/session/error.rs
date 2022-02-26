use std::io;

use fe2o3_amqp_types::definitions::{self, AmqpError, SessionError, Handle, ConnectionError, ErrorCondition};
use tokio::task::JoinError;

use crate::connection::AllocSessionError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error("All channels have been allocated")]
    ChannelMaxReached,

    #[error(transparent)]
    JoinError(#[from] JoinError),

    #[error("Local error {:?}", .0)]
    Local(definitions::Error),

    #[error("Remote error {:?}", .0)]
    Remote(definitions::Error),

    /// Link handle error should be handled differently. Link handle is only local
    #[error("Local LinkHandle {:?} error {:?}", .handle, .error)]
    LinkHandleError {
        handle: Handle,
        closed: bool,
        error: definitions::Error,
    },
}

impl Error {
    pub(crate) fn amqp_error(
        condition: impl Into<AmqpError>,
        description: impl Into<Option<String>>,
    ) -> Self {
        Self::Local(
            definitions::Error {
                condition: ErrorCondition::AmqpError(condition.into()),
                description: description.into(),
                info: None
            }
        )
    }

    pub(crate) fn session_error(
        condition: impl Into<SessionError>,
        description: impl Into<Option<String>>,
    ) -> Self {
        Self::Local(
            definitions::Error {
                condition: ErrorCondition::SessionError(condition.into()),
                description: description.into(),
                info: None
            }
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AllocLinkError {
    #[error("Illegal session state")]
    IllegalState,

    #[error("Reached session handle max")]
    HandleMaxReached,

    #[error("Link name must be unique")]
    DuplicatedLinkName,
}

impl From<AllocSessionError> for Error {
    fn from(err: AllocSessionError) -> Self {
        match err {
            AllocSessionError::Io(e) => Self::Io(e),
            AllocSessionError::ChannelMaxReached => Self::ChannelMaxReached,
            AllocSessionError::IllegalState => Self::Local(
                definitions::Error::new(AmqpError::IllegalState, None, None)
            ),
        }
    }
}

impl From<AllocLinkError> for definitions::Error {
    fn from(err: AllocLinkError) -> Self {
        match err {
            AllocLinkError::IllegalState => Self {
                condition: AmqpError::IllegalState.into(),
                description: None,
                info: None,
            },
            AllocLinkError::HandleMaxReached => Self {
                condition: ConnectionError::FramingError.into(),
                description: Some("Handle max has been reached".to_string()),
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

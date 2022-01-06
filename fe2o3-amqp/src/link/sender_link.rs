use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;
use fe2o3_amqp_types::{
    definitions::{
        self, AmqpError, DeliveryTag, Handle, MessageFormat, ReceiverSettleMode, Role,
        SenderSettleMode,
    },
    messaging::{DeliveryState, Source, Target},
    performatives::{Attach, Detach, Disposition, Transfer},
    primitives::Symbol,
};
use futures_util::{Sink, SinkExt};
use tokio::sync::{mpsc, oneshot, RwLock};

use crate::{
    endpoint::{self, Settlement},
    util::Consumer, link::state::SenderPermit,
};


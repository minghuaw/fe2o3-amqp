use fe2o3_amqp_types::transaction::Coordinator;

use crate::link::{sender::SenderInner, SenderFlowState, delivery::UnsettledMessage, Link, role};

pub(crate) type ControlLink = Link<role::Sender, Coordinator, SenderFlowState, UnsettledMessage>;

/// Transaction controller
#[derive(Debug)]
pub struct Controller {
    sender: SenderInner<ControlLink>,
}
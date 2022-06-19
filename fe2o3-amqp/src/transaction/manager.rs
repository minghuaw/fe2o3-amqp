//! Manages incoming transaction on the resource side

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions,
    performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer},
    transaction::TransactionId,
};
use futures_util::Sink;
use tokio::sync::mpsc;

use crate::{
    endpoint::{self, IncomingChannel, InputHandle, LinkFlow, OutgoingChannel, OutputHandle},
    link::{AttachError, LinkFrame, LinkRelay},
    session::frame::SessionFrame,
    Payload, Session,
};

use super::frame::TransactionalWork;

#[async_trait]
pub(crate) trait HandleControlLink {
    type Error: Send;

    async fn on_incoming_control_attach(
        &mut self,
        channel: IncomingChannel,
        attach: Attach,
    ) -> Result<(), Self::Error>;

    fn on_incoming_control_detach(
        &mut self,
        channel: IncomingChannel,
        detach: Detach,
    ) -> Result<(), Self::Error>;
}

/// How an incoming transaction should be handled in a session
#[cfg(feature = "transaction")]
pub(crate) trait ResourceTxnManager {
    type Error: Send;

    fn allocate_transaction_id(&mut self) -> Result<TransactionId, Self::Error>;

    fn commit_transaction(&mut self, txn_id: TransactionId) -> Result<(), Self::Error>;

    fn rollback_transaction(&mut self, txn_id: TransactionId) -> Result<(), Self::Error>;

    fn on_incoming_txn_posting(&mut self, transfer: Transfer) -> Result<(), Self::Error>;

    fn on_incoming_txn_retirement(&mut self, disposition: Disposition) -> Result<(), Self::Error>;

    fn on_incoming_txn_acquisition(&mut self, flow: Flow) -> Result<(), Self::Error>;
}

/// Transaction manager
#[derive(Debug)]
pub(crate) struct TransactionManager {
    pub control_link_tx: mpsc::Sender<LinkFrame>,
    pub txn_id_source: u64,
    pub txns: BTreeMap<TransactionId, TransactionalWork>,
    pub coordinators: BTreeSet<()>,
}

impl TransactionManager {}

///
#[derive(Debug)]
pub(crate) struct TxnSession<S>
where
    S: endpoint::Session,
{
    session: S,
    txn_manager: TransactionManager,
}

#[async_trait]
impl HandleControlLink for TxnSession<Session> {
    type Error = AttachError;

    async fn on_incoming_control_attach(
        &mut self,
        channel: IncomingChannel,
        attach: Attach,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn on_incoming_control_detach(
        &mut self,
        channel: IncomingChannel,
        detach: Detach,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}

#[async_trait]
impl<S> endpoint::Session for TxnSession<S>
where
    S: endpoint::Session + Send,
{
    type AllocError = S::AllocError;

    type Error = S::Error;

    type State = S::State;

    fn local_state(&self) -> &Self::State {
        todo!()
    }
    fn local_state_mut(&mut self) -> &mut Self::State {
        todo!()
    }
    fn outgoing_channel(&self) -> OutgoingChannel {
        todo!()
    }

    // Allocate new local handle for new Link
    fn allocate_link(
        &mut self,
        link_name: String,
        link_relay: Option<LinkRelay<()>>, // TODO: how to expose error at compile time?
    ) -> Result<OutputHandle, Self::AllocError> {
        todo!()
    }

    fn allocate_incoming_link(
        &mut self,
        link_name: String,
        link_relay: LinkRelay<()>,
        input_handle: InputHandle,
    ) -> Result<OutputHandle, Self::AllocError> {
        todo!()
    }

    fn deallocate_link(&mut self, output_handle: OutputHandle) {
        todo!()
    }

    fn on_incoming_begin(
        &mut self,
        channel: IncomingChannel,
        begin: Begin,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_attach(
        &mut self,
        channel: IncomingChannel,
        attach: Attach,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_flow(
        &mut self,
        channel: IncomingChannel,
        flow: Flow,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_transfer(
        &mut self,
        channel: IncomingChannel,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_disposition(
        &mut self,
        channel: IncomingChannel,
        disposition: Disposition,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_detach(
        &mut self,
        channel: IncomingChannel,
        detach: Detach,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_end(
        &mut self,
        channel: IncomingChannel,
        end: End,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    // Handling SessionFrames
    async fn send_begin<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<SessionFrame> + Send + Unpin,
    {
        todo!()
    }

    async fn send_end<W>(
        &mut self,
        writer: &mut W,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::Error>
    where
        W: Sink<SessionFrame> + Send + Unpin,
    {
        todo!()
    }

    // Intercepting LinkFrames
    fn on_outgoing_attach(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error> {
        todo!()
    }

    fn on_outgoing_flow(&mut self, flow: LinkFlow) -> Result<SessionFrame, Self::Error> {
        todo!()
    }

    fn on_outgoing_transfer(
        &mut self,
        input_handle: InputHandle,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<SessionFrame, Self::Error> {
        todo!()
    }

    fn on_outgoing_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error> {
        todo!()
    }

    fn on_outgoing_detach(&mut self, detach: Detach) -> Result<SessionFrame, Self::Error> {
        todo!()
    }
}

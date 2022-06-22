//! Implements session that can handle transaction

use async_trait::async_trait;
use fe2o3_amqp_types::{performatives::{Attach, Detach, Transfer, Flow, Disposition, Begin, End}, transaction::TransactionId, primitives::Symbol, messaging::DeliveryState, definitions};
use futures_util::Sink;
use tracing::instrument;
use uuid::Uuid;

use crate::{endpoint::{self, IncomingChannel, LinkFlow, OutgoingChannel, OutputHandle, InputHandle}, session::{self, frame::SessionFrame}, Payload, link::{LinkRelay, target_archetype::VariantOfTargetArchetype}};

use super::{manager::{TransactionManager, HandleControlLink, HandleTransactionalWork, ResourceTransaction}, TXN_ID_KEY, frame::TxnWorkFrame};

///
#[derive(Debug)]
pub(crate) struct TxnSession<S>
where
    S: endpoint::Session,
{
    pub(crate) session: S,
    pub(crate) txn_manager: TransactionManager,
}

#[async_trait]
impl<S> HandleControlLink for TxnSession<S>
where
    S: endpoint::Session<Error = session::Error> + endpoint::SessionExt + Send + Sync,
{
    type Error = S::Error;

    async fn on_incoming_control_attach(
        &mut self,
        channel: IncomingChannel,
        remote_attach: Attach,
    ) -> Result<(), Self::Error> {
        let coordinator = self
            .txn_manager
            .control_link_acceptor
            .accept_incoming_attach(
                remote_attach,
                self.session.control(),
                &self.txn_manager.control_link_outgoing,
            )
            .await
            .map_err(session::Error::CoordinatorAttachError)?;

        // TODO: store the joinhandle?
        let _ = tokio::spawn(coordinator.event_loop());
        Ok(())
    }
}

impl<S> endpoint::HandleDeclare for TxnSession<S>
where
    S: endpoint::Session<Error = session::Error> + endpoint::SessionExt + Send + Sync,
{
    fn allocate_transaction_id(&mut self) -> Result<TransactionId, Self::Error> {   
        let mut txn_id = TransactionId::from(Uuid::new_v4().into_bytes());
        while self.txn_manager.txns.contains_key(&txn_id) { // TODO: timeout?
            txn_id = TransactionId::from(Uuid::new_v4().into_bytes());
        }

        let _ = self.txn_manager.txns.insert(txn_id.clone(), ResourceTransaction::new());
        Ok(txn_id)
    }
}

impl<S> endpoint::HandleDischarge for TxnSession<S>
where
    S: endpoint::Session<Error = session::Error> + endpoint::SessionExt + Send + Sync,
{
    fn commit_transaction(&mut self, txn_id: TransactionId) -> Result<(), Self::Error> {
        todo!()
    }

    fn rollback_transaction(&mut self, txn_id: TransactionId) -> Result<(), Self::Error> {
        todo!()
    }
}

#[async_trait]
impl<S> HandleTransactionalWork for TxnSession<S>
where
    S: endpoint::Session<Error = session::Error> + Send,
{
    type Error = S::Error;

    async fn on_incoming_txn_transfer(
        &mut self,
        channel: IncomingChannel,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<(), Self::Error> {
        let txn_id = match &transfer.state {
            Some(DeliveryState::TransactionalState(txn_state)) => &txn_state.txn_id, 
            Some(_)
            | None => todo!(),
        };

        let txn = match self.txn_manager.txns.get_mut(txn_id) {
            Some(txn) => txn,
            None => todo!(), // TODO: Ignore?
        };

        let work_frame = TxnWorkFrame::Post { transfer, payload };
        txn.frames.push(work_frame);

        Ok(())
    }

    async fn on_incoming_txn_flow(
        &mut self,
        channel: IncomingChannel,
        flow: Flow,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    async fn on_incoming_txn_disposition(
        &mut self,
        channel: IncomingChannel,
        disposition: Disposition,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn on_outgoing_txn_transfer(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error> {
        todo!()
    }

    fn on_outgoing_txn_flow(&mut self, flow: LinkFlow) -> Result<SessionFrame, Self::Error> {
        todo!()
    }

    fn on_outgoing_txn_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error> {
        todo!()
    }
}

#[async_trait]
impl<S> endpoint::Session for TxnSession<S>
where
    S: endpoint::Session<Error = session::Error> + endpoint::SessionExt + Send + Sync,
{
    type AllocError = S::AllocError;

    type Error = S::Error;

    type State = S::State;

    fn local_state(&self) -> &Self::State {
        self.session.local_state()
    }
    fn local_state_mut(&mut self) -> &mut Self::State {
        self.session.local_state_mut()
    }
    fn outgoing_channel(&self) -> OutgoingChannel {
        self.session.outgoing_channel()
    }

    // Allocate new local handle for new Link
    fn allocate_link(
        &mut self,
        link_name: String,
        link_relay: Option<LinkRelay<()>>, // TODO: how to expose error at compile time?
    ) -> Result<OutputHandle, Self::AllocError> {
        self.session.allocate_link(link_name, link_relay)
    }

    fn allocate_incoming_link(
        &mut self,
        link_name: String,
        link_relay: LinkRelay<()>,
        input_handle: InputHandle,
    ) -> Result<OutputHandle, Self::AllocError> {
        self.session
            .allocate_incoming_link(link_name, link_relay, input_handle)
    }

    fn deallocate_link(&mut self, output_handle: OutputHandle) {
        self.session.deallocate_link(output_handle)
    }

    fn on_incoming_begin(
        &mut self,
        channel: IncomingChannel,
        begin: Begin,
    ) -> Result<(), Self::Error> {
        self.session.on_incoming_begin(channel, begin)
    }

    async fn on_incoming_attach(
        &mut self,
        channel: IncomingChannel,
        attach: Attach,
    ) -> Result<(), Self::Error> {
        match attach.target.as_ref().map(|t| t.is_coordinator()) {
            Some(true) => self.on_incoming_control_attach(channel, attach).await,
            Some(false) | None => self.session.on_incoming_attach(channel, attach).await,
        }
    }

    #[instrument(skip_all, flow = ?flow)]
    async fn on_incoming_flow(
        &mut self,
        channel: IncomingChannel,
        flow: Flow,
    ) -> Result<(), Self::Error> {
        match flow.properties.as_ref()
            .map(|fields| fields.contains_key(&Symbol::from(TXN_ID_KEY)))
        {
            Some(true) => self.on_incoming_txn_flow(channel, flow).await,
            Some(false)
            | None => self.session.on_incoming_flow(channel, flow).await,
        }
    }

    async fn on_incoming_transfer(
        &mut self,
        channel: IncomingChannel,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<(), Self::Error> {
        match &transfer.state {
            Some(DeliveryState::TransactionalState(_)) => self.on_incoming_txn_transfer(channel, transfer, payload).await,
            Some(_)
            | None => self.session.on_incoming_transfer(channel, transfer, payload).await,
        }
    }

    async fn on_incoming_disposition(
        &mut self,
        channel: IncomingChannel,
        disposition: Disposition,
    ) -> Result<(), Self::Error> {
        match disposition.state {
            Some(DeliveryState::TransactionalState(_)) => self.on_incoming_txn_disposition(channel, disposition).await,
            Some(_)
            | None => self.session.on_incoming_disposition(channel, disposition).await,
        }
    }

    async fn on_incoming_detach(
        &mut self,
        channel: IncomingChannel,
        detach: Detach,
    ) -> Result<(), Self::Error> {
        self.session.on_incoming_detach(channel, detach).await
    }

    async fn on_incoming_end(
        &mut self,
        channel: IncomingChannel,
        end: End,
    ) -> Result<(), Self::Error> {
        self.session.on_incoming_end(channel, end).await
    }

    // Handling SessionFrames
    async fn send_begin<W>(&mut self, writer: &mut W) -> Result<(), Self::Error>
    where
        W: Sink<SessionFrame> + Send + Unpin,
    {
        self.session.send_begin(writer).await
    }

    async fn send_end<W>(
        &mut self,
        writer: &mut W,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::Error>
    where
        W: Sink<SessionFrame> + Send + Unpin,
    {
        self.session.send_end(writer, error).await
    }

    // Intercepting LinkFrames
    fn on_outgoing_attach(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error> {
        self.session.on_outgoing_attach(attach)
    }

    fn on_outgoing_flow(&mut self, flow: LinkFlow) -> Result<SessionFrame, Self::Error> {
        // TODO:
        self.session.on_outgoing_flow(flow)
    }

    fn on_outgoing_transfer(
        &mut self,
        input_handle: InputHandle,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<SessionFrame, Self::Error> {
        // TODO:

        self.session.on_outgoing_transfer(input_handle, transfer, payload)
    }

    fn on_outgoing_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error> {
        // TODO:

        self.session.on_outgoing_disposition(disposition)
    }

    fn on_outgoing_detach(&mut self, detach: Detach) -> Result<SessionFrame, Self::Error> {
        self.session.on_outgoing_detach(detach)
    }
}

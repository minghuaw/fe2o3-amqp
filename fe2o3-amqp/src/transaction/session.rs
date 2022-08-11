//! Implements session that can handle transaction

use async_trait::async_trait;
use fe2o3_amqp_types::{
    definitions::{self},
    messaging::{Accepted, DeliveryState},
    performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer},
    transaction::{TransactionError, TransactionId},
};
use tokio::sync::{mpsc, oneshot};
use tracing::instrument;
use uuid::Uuid;

use crate::{
    control::SessionControl,
    endpoint::{self, IncomingChannel, InputHandle, LinkFlow, OutgoingChannel, OutputHandle},
    link::{target_archetype::VariantOfTargetArchetype, LinkRelay},
    session::{self, frame::SessionFrame},
    Payload,
};

use super::{
    frame::TxnWorkFrame,
    manager::{HandleControlLink, ResourceTransaction, TransactionManager},
    AllocTxnIdError, DischargeError,
};

pub(crate) async fn allocate_transaction_id(
    control: &mpsc::Sender<SessionControl>,
) -> Result<TransactionId, AllocTxnIdError> {
    let (resp, result) = oneshot::channel();

    control
        .send(SessionControl::AllocateTransactionId { resp })
        .await
        .map_err(|_| AllocTxnIdError::InvalidSessionState)?;
    result
        .await
        .map_err(|_| AllocTxnIdError::InvalidSessionState)?
}

pub(crate) async fn rollback_transaction(
    control: &mpsc::Sender<SessionControl>,
    txn_id: TransactionId,
) -> Result<Accepted, DischargeError> {
    let (resp, result) = oneshot::channel();

    control
        .send(SessionControl::RollbackTransaction { txn_id, resp })
        .await
        .map_err(|_| DischargeError::InvalidSessionState)?;
    result
        .await
        .map_err(|_| DischargeError::InvalidSessionState)?
        .map_err(Into::into)
}

pub(crate) async fn commit_transaction(
    control: &mpsc::Sender<SessionControl>,
    txn_id: TransactionId,
) -> Result<Accepted, DischargeError> {
    let (resp, result) = oneshot::channel();

    control
        .send(SessionControl::CommitTransaction { txn_id, resp })
        .await
        .map_err(|_| DischargeError::InvalidSessionState)?;
    result
        .await
        .map_err(|_| DischargeError::InvalidSessionState)?
        .map_err(Into::into)
}

///
#[derive(Debug)]
pub(crate) struct TxnSession<S>
where
    S: endpoint::Session,
{
    pub(crate) control: mpsc::Sender<SessionControl>,
    pub(crate) session: S,
    pub(crate) txn_manager: TransactionManager,
}

impl<S> TxnSession<S> where
    S: endpoint::Session<Error = session::Error> + endpoint::SessionExt + Send + Sync
{
}

#[async_trait]
impl<S> HandleControlLink for TxnSession<S>
where
    S: endpoint::Session<Error = session::Error> + endpoint::SessionExt + Send + Sync,
{
    type Error = S::Error;

    async fn on_incoming_control_attach(
        &mut self,
        remote_attach: Attach,
    ) -> Result<(), Self::Error> {
        let acceptor = self.txn_manager.control_link_acceptor.clone();
        let control = self.control.clone();
        let outgoing = self.txn_manager.control_link_outgoing.clone();

        tokio::spawn(async move {
            // Error accepting new control link is handled by acceptor
            if let Ok(coordinator) = acceptor
                .accept_incoming_attach(remote_attach, control, outgoing)
                .await
            {
                coordinator.event_loop().await
            }
        });

        Ok(())
    }
}

impl<S> endpoint::HandleDeclare for TxnSession<S>
where
    S: endpoint::Session<Error = session::Error> + endpoint::SessionExt + Send + Sync,
{
    fn allocate_transaction_id(&mut self) -> Result<TransactionId, AllocTxnIdError> {
        let mut txn_id = TransactionId::from(Uuid::new_v4().into_bytes());
        while self.txn_manager.txns.contains_key(&txn_id) {
            // TODO: timeout?
            txn_id = TransactionId::from(Uuid::new_v4().into_bytes());
        }

        let _ = self
            .txn_manager
            .txns
            .insert(txn_id.clone(), ResourceTransaction::new());
        Ok(txn_id)
    }
}

#[async_trait]
impl<S> endpoint::HandleDischarge for TxnSession<S>
where
    S: endpoint::Session<Error = session::Error> + endpoint::SessionExt + Send + Sync,
{
    #[instrument(skip_all)]
    async fn commit_transaction(
        &mut self,
        txn_id: TransactionId,
    ) -> Result<Result<Accepted, TransactionError>, Self::Error> {
        let txn = match self.txn_manager.txns.remove(&txn_id) {
            Some(txn) => txn,
            None => return Ok(Err(TransactionError::UnknownId)),
        };

        for work_frame in txn.frames {
            match work_frame {
                TxnWorkFrame::Post {
                    mut transfer,
                    payload,
                } => {
                    // FIXME: This informs the controller of the outcome that will be in effect at the point that the
                    // transaction is successfully discharged
                    if let Some(DeliveryState::TransactionalState(txn_state)) = transfer.state {
                        transfer.state = txn_state.outcome.map(Into::into);
                    };

                    // Committing shuold never need to send an immediate disposition
                    if let Some(disposition) =
                        self.session.on_incoming_transfer(transfer, payload).await?
                    {
                        self.control
                            .send(SessionControl::Disposition(disposition))
                            .await
                            .map_err(|_| Self::Error::IllegalState)?
                    }
                }
                TxnWorkFrame::Retire(mut disposition) => {
                    // On a successful discharge, the resource will apply the given outcome and can immediately settle the transfers.
                    if let Some(DeliveryState::TransactionalState(txn_state)) = disposition.state {
                        disposition.state = txn_state.outcome.map(Into::into)
                    }
                    disposition.settled = true;

                    // TODO: Where should the echoing disposition be sent?
                    if let Some(dispositions) =
                        self.session.on_incoming_disposition(disposition).await?
                    {
                        for disposition in dispositions {
                            self.control
                                .send(SessionControl::Disposition(disposition))
                                .await
                                .map_err(|_| Self::Error::IllegalState)?
                        }
                    }
                }
            }
        }

        Ok(Ok(Accepted {}))
    }

    fn rollback_transaction(
        &mut self,
        txn_id: TransactionId,
    ) -> Result<Result<Accepted, TransactionError>, Self::Error> {
        match self.txn_manager.txns.remove(&txn_id) {
            Some(_) => {
                // TODO: Simply drop the frames?
                Ok(Ok(Accepted {}))
            }
            None => Ok(Err(TransactionError::UnknownId)),
        }
    }
}

#[async_trait]
impl<S> endpoint::Session for TxnSession<S>
where
    S: endpoint::Session<Error = session::Error> + endpoint::SessionExt + Send + Sync,
{
    type AllocError = S::AllocError;
    type BeginError = S::BeginError;
    type EndError = S::EndError;
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
    ) -> Result<(), Self::BeginError> {
        self.session.on_incoming_begin(channel, begin)
    }

    async fn on_incoming_attach(&mut self, attach: Attach) -> Result<(), Self::Error> {
        match attach.target.as_ref().map(|t| t.is_coordinator()) {
            Some(true) => self.on_incoming_control_attach(attach).await,
            Some(false) | None => self.session.on_incoming_attach(attach).await,
        }
    }

    #[instrument(skip_all, flow = ?flow)]
    async fn on_incoming_flow(&mut self, flow: Flow) -> Result<Option<LinkFlow>, Self::Error> {
        // TODO: implement transactional acquisition
        // match flow
        //     .properties
        //     .as_ref()
        //     .map(|fields| fields.contains_key(TXN_ID_KEY))
        // {
        //     Some(true) => {
        //         // self.on_incoming_txn_flow(flow).await
        //     }
        //     Some(false) | None => self.session.on_incoming_flow(flow).await,
        // }
        self.session.on_incoming_flow(flow).await
    }

    async fn on_incoming_transfer(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Option<Disposition>, Self::Error> {
        let (txn, txn_id) = match &transfer.state {
            Some(DeliveryState::TransactionalState(state)) => {
                let txn_id = &state.txn_id;
                self.txn_manager
                    .txns
                    .get_mut(txn_id)
                    .map(|txn| (txn, txn_id.clone()))
                    .ok_or(S::Error::UnknownTxnId)?
            }
            Some(_) | None => return self.session.on_incoming_transfer(transfer, payload).await,
        };

        // if let Some(disposition) = txn.on_incoming_post(txn_id, transfer, payload) {
        //     self.session
        //         .control()
        //         .send(SessionControl::Disposition(disposition))
        //         .await
        //         .map_err(|_| S::Error::IllegalConnectionState)?;
        // }
        // Ok(())
        Ok(txn.on_incoming_post(txn_id, transfer, payload))
    }

    async fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<Option<Vec<Disposition>>, Self::Error> {
        match &disposition.state {
            Some(DeliveryState::TransactionalState(state)) => {
                let txn_id = &state.txn_id;
                match self.txn_manager.txns.get_mut(txn_id) {
                    Some(txn) => {
                        txn.frames.push(TxnWorkFrame::Retire(disposition));
                        Ok(None) // TODO: need to consider the receiver settle mode?
                    }
                    None => {
                        // FIXME: Should this stop the session or just the coordinator associated with the txn?
                        Err(S::Error::UnknownTxnId)
                    }
                }
            }
            Some(_) | None => self.session.on_incoming_disposition(disposition).await,
        }
    }

    async fn on_incoming_detach(&mut self, detach: Detach) -> Result<(), Self::Error> {
        self.session.on_incoming_detach(detach).await
    }

    async fn on_incoming_end(
        &mut self,
        channel: IncomingChannel,
        end: End,
    ) -> Result<(), Self::EndError> {
        self.session.on_incoming_end(channel, end).await
    }

    // Handling SessionFrames
    async fn send_begin(
        &mut self,
        writer: &mpsc::Sender<SessionFrame>,
    ) -> Result<(), Self::BeginError> {
        self.session.send_begin(writer).await
    }

    async fn send_end(
        &mut self,
        writer: &mpsc::Sender<SessionFrame>,
        error: Option<definitions::Error>,
    ) -> Result<(), Self::EndError> {
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

        self.session
            .on_outgoing_transfer(input_handle, transfer, payload)
    }

    fn on_outgoing_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error> {
        // TODO:

        self.session.on_outgoing_disposition(disposition)
    }

    fn on_outgoing_detach(&mut self, detach: Detach) -> SessionFrame {
        self.session.on_outgoing_detach(detach)
    }
}

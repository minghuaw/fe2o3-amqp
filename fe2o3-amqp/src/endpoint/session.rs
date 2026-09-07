//! Defines traits for session implementations

use std::future::Future;

use std::sync::{Arc, OnceLock};

use fe2o3_amqp_types::{
    definitions::Error,
    performatives::{Attach, Begin, Detach, Disposition, End, Flow, Transfer},
};

use tokio::sync::mpsc;

use crate::{
    connection::ConnectionStopReason,
    link::{LinkRelay, SessionStopReason},
    session::frame::{SessionFrame, SessionOutgoingItem},
    Payload, SendBound,
};

use super::{IncomingChannel, InputHandle, LinkFlow, OutgoingChannel, OutputHandle};

pub(crate) trait Session {
    type AllocError: SendBound;
    type BeginError: SendBound;
    type EndError: SendBound;
    type Error: SendBound;
    type State;

    fn local_state(&self) -> &Self::State;

    /// Record why the session (or its connection) stopped
    ///
    /// Only succeeds if the stop reason has not been recorded yet; a later
    /// call is a no-op (the first recorded reason wins).
    fn set_session_stop_reason(&mut self, reason: SessionStopReason);

    /// The shared cell holding why the session (or its connection) stopped
    fn session_stop_reason(&self) -> &Arc<OnceLock<SessionStopReason>>;

    /// The shared cell holding why the connection stopped
    fn connection_stop_reason(&self) -> &Arc<OnceLock<ConnectionStopReason>>;

    fn outgoing_channel(&self) -> OutgoingChannel;

    // Allocate new local handle for new Link
    fn allocate_link(
        &mut self,
        link_name: String,
        link_relay: Option<LinkRelay<()>>,
    ) -> Result<OutputHandle, Self::AllocError>;

    fn allocate_incoming_link(
        &mut self,
        link_name: String,
        link_relay: LinkRelay<()>,
        input_handle: InputHandle,
    ) -> Result<OutputHandle, Self::AllocError>;

    /// Release the link's bookkeeping (name and output handle). Returns
    /// whether the bookkeeping was still present.
    fn deallocate_link(&mut self, output_handle: OutputHandle) -> bool;

    fn on_incoming_begin(
        &mut self,
        channel: IncomingChannel,
        begin: Begin,
    ) -> Result<(), Self::BeginError>;

    fn on_incoming_attach(
        &mut self,
        attach: Attach,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// An `Ok(Some(link_flow))` means an immediate echo of the link flow is requested
    fn on_incoming_flow(
        &mut self,
        flow: Flow,
    ) -> impl Future<Output = Result<Option<SessionOutgoingItem>, Self::Error>> + Send;

    /// Handle an incoming transfer.
    ///
    /// An `Ok(Some(Disposition))` is produced only by the transactional session
    /// implementation ([`crate::transaction::session::TxnSession`]) as the
    /// presumptive-outcome reply required when a posted transfer is received
    /// (AMQP §4.4.1). Non-transactional implementations MUST return `Ok(None)`,
    /// since settlement of ordinary deliveries is handled at the link level.
    /// The session engine sends the reply in the live path; the transaction
    /// discharge path consumes it via `SessionControl::Disposition`.
    fn on_incoming_transfer(
        &mut self,
        transfer: Transfer,
        payload: Payload,
    ) -> impl Future<Output = Result<Option<Disposition>, Self::Error>> + Send;

    /// An `Ok(Some(Disposition))` means an immediate disposition should be sent back
    fn on_incoming_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<Option<Vec<Disposition>>, Self::Error>;

    /// Handle an incoming detach, returning the response detach for the peer
    /// (the relay's reply to a remote-initiated detach) when one is owed. The
    /// response is not sent here: the engine routes it through
    /// [`Session::on_outgoing_detach`] with `expects_echo = false`.
    fn on_incoming_detach(
        &mut self,
        detach: Detach,
    ) -> impl Future<Output = Result<Option<Detach>, Self::Error>> + Send;

    fn on_incoming_end(&mut self, channel: IncomingChannel, end: End)
        -> Result<(), Self::EndError>;

    // Handling SessionFrames
    async fn send_begin(
        &mut self,
        writer: &mpsc::Sender<SessionFrame>,
    ) -> Result<(), Self::BeginError>;

    fn send_end(
        &mut self,
        writer: &mpsc::Sender<SessionFrame>,
        error: Option<Error>,
    ) -> impl Future<Output = Result<(), Self::EndError>> + Send;

    // Intercepting LinkFrames
    fn on_outgoing_attach(&mut self, attach: Attach) -> Result<SessionFrame, Self::Error>;

    fn on_outgoing_flow(&mut self, flow: LinkFlow) -> Result<SessionFrame, Self::Error>;

    /// Returns a session-only flow (no link handle) when the session should proactively
    /// re-advertise its window after receiving transfers. Called by the session engine after
    /// every incoming transfer; returns `None` when no flow is due.
    fn maybe_outgoing_session_flow(&mut self) -> Option<SessionOutgoingItem>;

    fn on_outgoing_transfer(
        &mut self,
        input_handle: InputHandle,
        transfer: Transfer,
        payload: Payload,
    ) -> Result<Option<SessionOutgoingItem>, Self::Error>;

    fn on_outgoing_disposition(
        &mut self,
        disposition: Disposition,
    ) -> Result<SessionFrame, Self::Error>;

    /// Send a detach frame out, releasing the link's bookkeeping, and record
    /// whether the peer's response detach is expected.
    ///
    /// `expects_echo` is true for a locally initiated detach (engine-written
    /// close/detach/drop/attach-error): the output handle is recorded so an
    /// incoming detach on the link can be recognized as the peer's echo. It
    /// is false for the relay's reply to a remote-initiated detach, for which
    /// the peer sends nothing back. Returns `None` when a locally initiated
    /// detach is a duplicate (the link's bookkeeping is already gone because
    /// the relay answered the remote's detach first); nothing is sent then.
    fn on_outgoing_detach(&mut self, detach: Detach, expects_echo: bool) -> Option<SessionFrame>;
}

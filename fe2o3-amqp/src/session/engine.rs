use tokio::{sync::mpsc::{Receiver, Sender}, task::JoinHandle};

use crate::{control::SessionControl, endpoint, error::EngineError};

use super::{SessionFrame, SessionFrameBody, SessionState};

pub struct Engine<S> {
    session: S,
    control: Receiver<SessionControl>,
    incoming: Receiver<Result<SessionFrame, EngineError>>,
    outgoing: Sender<SessionFrame>,

    // outgoing_link_frames : Receiver<LinkFrame>,
}

impl<S> Engine<S> 
where 
    S: endpoint::Session<State = SessionState> + Send + 'static
{
    pub fn new(
        session: S,
        control: Receiver<SessionControl>,
        incoming: Receiver<Result<SessionFrame, EngineError>>,
        outgoing: Sender<SessionFrame>,
    ) -> Self {
        Self {
            session,
            control,
            incoming,
            outgoing
        }
    }

    pub fn spawn(self) ->JoinHandle<Result<(), EngineError>> {
        tokio::spawn(self.event_loop())
    }

    #[inline]
    async fn on_incoming(&mut self, incoming: Result<SessionFrame, EngineError>) -> Result<(), EngineError> {
        let SessionFrame { channel, body } = incoming?;

        match body {
            SessionFrameBody::Begin(begin) => {
                self.session.on_incoming_begin(begin).await
                    .map_err(Into::into)?;
            },
            SessionFrameBody::Attach(attach) => {
                self.session.on_incoming_attach(attach).await
                    .map_err(Into::into)?;
            },
            SessionFrameBody::Flow(flow) => {
                self.session.on_incoming_flow(flow).await
                    .map_err(Into::into)?;
            },
            SessionFrameBody::Transfer{performative, payload} => {
                self.session.on_incoming_transfer(performative, payload).await
                    .map_err(Into::into)?;
            },
            SessionFrameBody::Disposition(disposition) => {
                self.session.on_incoming_disposition(disposition).await
                    .map_err(Into::into)?;
            },
            SessionFrameBody::Detach(detach) => {
                self.session.on_incoming_detach(detach).await
                    .map_err(Into::into)?;
            },
            SessionFrameBody::End(end) => {
                self.session.on_incoming_end(end).await
                    .map_err(Into::into)?;
            }
        }
        Ok(())
    }

    #[inline]
    async fn on_control(&mut self, control: SessionControl) -> Result<(), EngineError> {
        match control {
            SessionControl::Begin => {
                todo!()
            },
            SessionControl::End => {
                todo!()
            }
        }
    }

    async fn event_loop(mut self) -> Result<(), EngineError> {
        loop {
            let result = tokio::select! {
                incoming = self.incoming.recv() => {
                    match incoming {
                        Some(incoming) => self.on_incoming(incoming).await,
                        None => todo!()
                    }
                },
                control = self.control.recv() => {
                    match control {
                        Some(control) => self.on_control(control).await,
                        None => todo!()
                    }
                }
            };


        }

        Ok(())
    }
}


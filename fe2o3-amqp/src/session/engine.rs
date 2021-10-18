use tokio::{sync::mpsc::{Receiver, Sender}, task::JoinHandle};

use crate::{control::SessionControl, endpoint, error::EngineError, session::Session, util::Running};

use super::{SessionFrame, SessionFrameBody, SessionState};

pub struct SessionEngine<S> {
    session: S,
    control: Receiver<SessionControl>,
    incoming: Receiver<Result<SessionFrame, EngineError>>,
    outgoing: Sender<SessionFrame>,

    // outgoing_link_frames : Receiver<LinkFrame>,
}

impl<S> SessionEngine<S> 
where 
    S: endpoint::Session<State = SessionState> + Send + 'static
{
    // pub fn new(
    //     session: S,
    //     control: Receiver<SessionControl>,
    //     incoming: Receiver<Result<SessionFrame, EngineError>>,
    //     outgoing: Sender<SessionFrame>,
    // ) -> Self {
    //     Self {
    //         session,
    //         control,
    //         incoming,
    //         outgoing
    //     }
    // }

    pub async fn begin(
        session: S,
        control: Receiver<SessionControl>,
        incoming: Receiver<Result<SessionFrame, EngineError>>,
        outgoing: Sender<SessionFrame>,
    ) -> Result<Self, EngineError> {
        let mut engine = Self {
            session,
            control,
            incoming,
            outgoing
        };

        // send a begin
        engine.session.on_outgoing_begin(&mut engine.outgoing).await
            .map_err(Into::into)?;
        // wait for an incoming begin
        let frame = match engine.incoming.recv().await {
            Some(frame) => frame?,
            None => todo!()
        };
        let SessionFrame { channel, body } = frame;
        let remote_begin = match body {
            SessionFrameBody::Begin(begin) => begin,
            _ => return Err(EngineError::illegal_state())
        };
        engine.session.on_incoming_begin(channel, remote_begin).await
            .map_err(Into::into)?;
        Ok(engine)
    }

    pub fn spawn(self) ->JoinHandle<Result<(), EngineError>> {
        tokio::spawn(self.event_loop())
    }

    #[inline]
    async fn on_incoming(&mut self, incoming: Result<SessionFrame, EngineError>) -> Result<Running, EngineError> {
        let SessionFrame { channel, body } = incoming?;

        match body {
            SessionFrameBody::Begin(begin) => {
                self.session.on_incoming_begin(channel, begin).await
                    .map_err(Into::into)?;
            },
            SessionFrameBody::Attach(attach) => {
                self.session.on_incoming_attach(channel, attach).await
                    .map_err(Into::into)?;
            },
            SessionFrameBody::Flow(flow) => {
                self.session.on_incoming_flow(channel, flow).await
                    .map_err(Into::into)?;
            },
            SessionFrameBody::Transfer{performative, payload} => {
                self.session.on_incoming_transfer(channel, performative, payload).await
                    .map_err(Into::into)?;
            },
            SessionFrameBody::Disposition(disposition) => {
                self.session.on_incoming_disposition(channel, disposition).await
                    .map_err(Into::into)?;
            },
            SessionFrameBody::Detach(detach) => {
                self.session.on_incoming_detach(channel, detach).await
                    .map_err(Into::into)?;
            },
            SessionFrameBody::End(end) => {
                self.session.on_incoming_end(channel, end).await
                    .map_err(Into::into)?;
            }
        }

        match self.session.local_state() {
            SessionState::Unmapped => Ok(Running::Stop),
            _ => Ok(Running::Continue)
        }
    }

    #[inline]
    async fn on_control(&mut self, control: SessionControl) -> Result<Running, EngineError> {
        match control {
            SessionControl::Begin => {
                self.session.on_outgoing_begin(&mut self.outgoing).await
                    .map_err(Into::into)?;
            },
            SessionControl::End(error) => {
                self.session.on_outgoing_end(&mut self.outgoing, error).await
                    .map_err(Into::into)?;
            }
        }

        match self.session.local_state() {
            SessionState::Unmapped => Ok(Running::Stop),
            _ => Ok(Running::Continue)
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

            match result {
                Ok(running) => {
                    match running {
                        Running::Continue => {},
                        Running::Stop => break,
                    }
                },
                Err(err) => {
                    todo!()
                }
            }
        }

        println!(">>> Debug: SessionEngine exiting event_loop");

        Ok(())
    }
}


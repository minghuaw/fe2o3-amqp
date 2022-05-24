//! The engine handles incoming and outgoing frames and messages to reduce
//! transferring frames/messages over channels

use std::cmp::min;
use std::io;
use std::time::Duration;

use fe2o3_amqp_types::definitions::{self, AmqpError};
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tracing::{debug, error, instrument, trace};

use crate::control::ConnectionControl;
use crate::endpoint::{IncomingChannel, OutgoingChannel};
use crate::frames::amqp::{self, Frame, FrameBody};
use crate::session::frame::{SessionFrame, SessionFrameBody};
use crate::transport::Transport;
use crate::util::Running;
use crate::{endpoint, transport};

use super::{heartbeat::HeartBeat, ConnectionState, Error};
use super::{AllocSessionError, OpenError};

#[derive(Debug)]
pub(crate) struct ConnectionEngine<Io, C> {
    transport: Transport<Io, amqp::Frame>,
    connection: C,
    control: Receiver<ConnectionControl>,
    outgoing_session_frames: Receiver<SessionFrame>,
    heartbeat: HeartBeat,
}

impl<Io, C> ConnectionEngine<Io, C>
where
    Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static,
    C: endpoint::Connection<State = ConnectionState> + std::fmt::Debug + Send + 'static,
    C::Error: Into<Error> + From<transport::Error>,
    C::AllocError: Into<AllocSessionError>,
{
    async fn on_open_error(
        transport: &mut Transport<Io, amqp::Frame>,
        connection: &mut C,
        error: Error,
    ) -> OpenError {
        match error {
            Error::Io(err) => OpenError::Io(err),
            Error::JoinError(_) => unreachable!(),
            Error::Local(err) => {
                let _ = connection.send_close(transport, Some(err.clone())).await;
                OpenError::LocalError(err)
            }
            Error::Remote(err) => OpenError::RemoteError(err),
        }
    }

    /// Open Connection without starting the Engine::event_loop()
    pub(crate) async fn open(
        transport: Transport<Io, amqp::Frame>,
        connection: C,
        control: Receiver<ConnectionControl>,
        outgoing_session_frames: Receiver<SessionFrame>,
    ) -> Result<Self, OpenError> {
        let mut engine = Self {
            transport,
            connection,
            control,
            outgoing_session_frames,
            heartbeat: HeartBeat::never(),
        };

        // Send an Open
        if let Err(error) = engine.connection.send_open(&mut engine.transport).await {
            return Err(Self::on_open_error(
                &mut engine.transport,
                &mut engine.connection,
                error.into(),
            )
            .await);
        }

        // Wait for an Open
        let frame = match engine.transport.next().await {
            Some(frame) => match frame {
                Ok(fr) => fr,
                Err(error) => {
                    return Err(Self::on_open_error(
                        &mut engine.transport,
                        &mut engine.connection,
                        error.into(),
                    )
                    .await)
                }
            },
            None => {
                return Err(OpenError::Io(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Expecting an Open frame",
                )))
            }
        };
        let Frame { channel, body } = frame;
        let channel = endpoint::IncomingChannel(channel);
        let remote_open = match body {
            FrameBody::Open(open) => open,
            _ => {
                return Err(OpenError::LocalError(definitions::Error::new(
                    AmqpError::IllegalState,
                    None,
                    None,
                )))
            }
        };

        // Handle incoming remote_open
        let remote_max_frame_size = remote_open.max_frame_size.0;
        let remote_idle_timeout = remote_open.idle_time_out;
        if let Err(error) = engine
            .connection
            .on_incoming_open(channel, remote_open)
            .await
        {
            return Err(Self::on_open_error(
                &mut engine.transport,
                &mut engine.connection,
                error.into(),
            )
            .await);
        }

        // update transport setting
        let max_frame_size = min(
            engine.connection.local_open().max_frame_size.0,
            remote_max_frame_size,
        );
        engine.transport.set_max_frame_size(max_frame_size as usize);

        // Set heartbeat here because in pipelined-open, the Open frame
        // may be recved after mux loop is started
        match &remote_idle_timeout {
            Some(millis) => {
                let period = Duration::from_millis(*millis as u64);
                engine.heartbeat = HeartBeat::new(period);
            }
            None => engine.heartbeat = HeartBeat::never(),
        };

        Ok(engine)
    }

    pub fn spawn(self) -> JoinHandle<Result<(), Error>> {
        tokio::spawn(self.event_loop())
    }
}

impl<Io, C> ConnectionEngine<Io, C>
where
    Io: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin,
    C: endpoint::Connection<State = ConnectionState> + std::fmt::Debug + Send + 'static,
    C::Error: Into<Error> + From<transport::Error>,
    C::AllocError: Into<AllocSessionError>,
{
    #[instrument(skip_all)]
    async fn forward_to_session(&mut self, channel: IncomingChannel, frame: SessionFrame) -> Result<(), Error> {
        trace!(frame = ?frame);
        match &self.connection.local_state() {
            ConnectionState::Opened => {}
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)),
        };

        match self.connection.session_tx_by_incoming_channel(channel) {
            Some(tx) => tx.send(frame).await?,
            None => return Err(Error::amqp_error(AmqpError::NotFound, None)),
        };
        Ok(())
    }

    #[instrument(name = "RECV", skip_all)]
    async fn on_incoming(&mut self, incoming: Result<Frame, Error>) -> Result<Running, Error> {
        let frame = incoming?;

        let Frame { channel, body } = frame;
        let channel = IncomingChannel(channel);
        match body {
            FrameBody::Open(open) => {
                let remote_max_frame_size = open.max_frame_size.0;
                let remote_idle_timeout = open.idle_time_out;
                self.connection
                    .on_incoming_open(channel, open)
                    .await
                    .map_err(Into::into)?;

                // update transport setting
                let max_frame_size = min(
                    self.connection.local_open().max_frame_size.0,
                    remote_max_frame_size,
                );
                self.transport.set_max_frame_size(max_frame_size as usize);

                // Set heartbeat here because in pipelined-open, the Open frame
                // may be recved after mux loop is started
                match &remote_idle_timeout {
                    Some(millis) => {
                        let period = Duration::from_millis(*millis as u64);
                        self.heartbeat = HeartBeat::new(period);
                    }
                    None => self.heartbeat = HeartBeat::never(),
                };
            }
            FrameBody::Begin(begin) => {
                self.connection
                    .on_incoming_begin(channel, begin)
                    .await
                    .map_err(Into::into)?;
            }
            FrameBody::Attach(attach) => {
                let sframe = SessionFrame::new(channel, SessionFrameBody::Attach(attach));
                self.forward_to_session(channel, sframe).await?;
            }
            FrameBody::Flow(flow) => {
                let sframe = SessionFrame::new(channel, SessionFrameBody::Flow(flow));
                self.forward_to_session(channel, sframe).await?;
            }
            FrameBody::Transfer {
                performative,
                payload,
            } => {
                let sframe = SessionFrame::new(
                    channel,
                    SessionFrameBody::Transfer {
                        performative,
                        payload,
                    },
                );
                self.forward_to_session(channel, sframe).await?;
            }
            FrameBody::Disposition(disposition) => {
                let sframe = SessionFrame::new(channel, SessionFrameBody::Disposition(disposition));
                self.forward_to_session(channel, sframe).await?;
            }
            FrameBody::Detach(detach) => {
                let sframe = SessionFrame::new(channel, SessionFrameBody::Detach(detach));
                self.forward_to_session(channel, sframe).await?;
            }
            FrameBody::End(end) => {
                self.connection
                    .on_incoming_end(channel, end)
                    .await
                    .map_err(Into::into)?;
            }
            FrameBody::Close(close) => {
                self.connection
                    .on_incoming_close(channel, close)
                    .await
                    .map_err(Into::into)?;
            }
            FrameBody::Empty => {
                // do nothing, IdleTimeout is tracked by Transport
            }
        }

        match self.connection.local_state() {
            ConnectionState::End => Ok(Running::Stop),
            _ => Ok(Running::Continue),
        }
    }

    #[inline]
    #[instrument(skip_all)]
    async fn on_control(&mut self, control: ConnectionControl) -> Result<Running, Error> {
        debug!("{}", control);
        match control {
            ConnectionControl::Close(error) => {
                self.outgoing_session_frames.close();
                while let Some(frame) = self.outgoing_session_frames.recv().await {
                    self.on_outgoing_session_frames(frame).await?;
                }

                self.connection
                    .send_close(&mut self.transport, error)
                    .await
                    .map_err(Into::into)?;
            }
            ConnectionControl::AllocateSession { tx, responder } => {
                let result = self.connection.allocate_session(tx).map_err(Into::into);
                responder.send(result).map_err(|_| {
                    Error::Io(io::Error::new(
                        io::ErrorKind::Other,
                        "ConnectionHandle is dropped",
                    ))
                })?;
            }
            ConnectionControl::DeallocateSession(session_id) => {
                self.connection.deallocate_session(session_id)
            }
        }

        match self.connection.local_state() {
            ConnectionState::End => Ok(Running::Stop),
            _ => Ok(Running::Continue),
        }
    }

    #[inline]
    #[instrument(name = "SEND", skip_all)]
    async fn on_outgoing_session_frames(&mut self, frame: SessionFrame) -> Result<Running, Error> {
        use crate::frames::amqp::FrameBody;

        match self.connection.local_state() {
            ConnectionState::Opened => {}
            _ => return Err(Error::amqp_error(AmqpError::IllegalState, None)),
        }

        let SessionFrame { channel, body } = frame;
        let channel = OutgoingChannel(channel);
        let frame = match body {
            SessionFrameBody::Begin(begin) => self
                .connection
                .on_outgoing_begin(channel, begin)
                .map_err(Into::into)?,
            SessionFrameBody::Attach(attach) => {
                // trace!(channel, frame = ?attach);
                Frame::new(channel, FrameBody::Attach(attach))
            }
            SessionFrameBody::Flow(flow) => {
                // trace!(channel, frame = ?flow);
                Frame::new(channel, FrameBody::Flow(flow))
            }
            SessionFrameBody::Transfer {
                performative,
                payload,
            } => {
                // trace!(channel, frame = ?performative, payload_len = payload.len());
                Frame::new(
                    channel,
                    FrameBody::Transfer {
                        performative,
                        payload,
                    },
                )
            }
            SessionFrameBody::Disposition(disposition) => {
                // trace!(channel, frame = ?disposition);
                Frame::new(channel, FrameBody::Disposition(disposition))
            }
            SessionFrameBody::Detach(detach) => {
                // trace!(channel, frame = ?detach);
                Frame::new(channel, FrameBody::Detach(detach))
            }
            SessionFrameBody::End(end) => self
                .connection
                .on_outgoing_end(channel, end)
                .map_err(Into::into)?,
        };

        trace!(channel = frame.channel, frame = ?frame.body);
        self.transport.send(frame).await?;
        Ok(Running::Continue)
    }

    #[inline]
    async fn on_heartbeat(&mut self) -> Result<Running, Error> {
        match &self.connection.local_state() {
            ConnectionState::Start | ConnectionState::CloseSent => return Ok(Running::Continue),
            ConnectionState::End => return Ok(Running::Stop),
            _ => {}
        }

        let frame = Frame::empty();
        self.transport.send(frame).await?;
        Ok(Running::Continue)
    }

    #[inline]
    async fn on_error(&mut self, error: &Error) -> Running {
        match error {
            Error::Io(_) => Running::Stop,
            Error::JoinError(_) => unreachable!(), // JoinError is only for the event_loop task
            Error::Local(error) => {
                let _ = self
                    .connection
                    .send_close(&mut self.transport, Some(error.clone()))
                    .await;
                self.continue_or_stop_by_state()
            }
            Error::Remote(_) => {
                // TODO: Simplify remote error handling?
                self.continue_or_stop_by_state()
            }
        }
    }

    /// Get whether event loop should conditnue or stop by ConnectionState
    #[inline]
    fn continue_or_stop_by_state(&self) -> Running {
        match self.connection.local_state() {
            ConnectionState::Start
            | ConnectionState::HeaderReceived
            | ConnectionState::HeaderSent
            | ConnectionState::HeaderExchange
            | ConnectionState::OpenPipe
            | ConnectionState::OpenClosePipe
            | ConnectionState::OpenReceived
            | ConnectionState::OpenSent
            | ConnectionState::Opened
            | ConnectionState::CloseReceived
            | ConnectionState::CloseSent => Running::Continue,
            ConnectionState::ClosePipe | ConnectionState::Discarding | ConnectionState::End => {
                Running::Stop
            }
        }
    }

    #[instrument(name = "Connection::event_loop", skip(self), fields(container_id = %self.connection.local_open().container_id))]
    async fn event_loop(mut self) -> Result<(), Error> {
        let mut outcome = Ok(());
        loop {
            let result = tokio::select! {
                _ = self.heartbeat.next() => self.on_heartbeat().await,
                incoming = self.transport.next() => {
                    let result = match incoming {
                        Some(incoming) => self.on_incoming(incoming.map_err(Into::into)).await,
                        None => {
                            // Incoming stream is closed

                            match self.connection.local_state() {
                                ConnectionState::Start
                                | ConnectionState::HeaderReceived
                                | ConnectionState::HeaderSent
                                | ConnectionState::HeaderExchange
                                | ConnectionState::OpenPipe
                                | ConnectionState::OpenClosePipe
                                | ConnectionState::OpenReceived
                                | ConnectionState::OpenSent
                                | ConnectionState::Opened
                                | ConnectionState::CloseReceived
                                | ConnectionState::CloseSent => Err(Error::Io(io::Error::new(
                                    io::ErrorKind::UnexpectedEof,
                                    "Transport is closed before connection is closed"
                                ))),
                                ConnectionState::ClosePipe
                                | ConnectionState::Discarding
                                | ConnectionState::End => Ok(Running::Stop),
                            }
                        },
                    };

                    result
                },
                control = self.control.recv() => {
                    let result = match control {
                        Some(control) => self.on_control(control).await,
                        None => {
                            // All control channel are dropped (which is impossible)
                            Ok(Running::Stop)
                        }
                    };

                    result
                },
                frame = self.outgoing_session_frames.recv() => {
                    let result = match frame {
                        Some(frame) => self.on_outgoing_session_frames(frame).await,
                        None => {
                            // Upon closing, the outgoing_session_frames channel will be closed
                            // first while the connection may still be waiting for remote
                            // close frame.
                            Ok(Running::Continue)
                        }
                    };

                    result
                }
            };

            let running = match result {
                Ok(running) => running,
                Err(error) => {
                    // TODO: error handling
                    error!("{:?}", error);
                    let running = self.on_error(&error).await;
                    outcome = Err(error);
                    running
                }
            };

            match running {
                Running::Continue => {}
                Running::Stop => break,
            }
        }

        // Clean Shutdown
        //
        // When the Receiver is dropped, it is possible for unprocessed messages to remain
        // in the channel. Instead, it is usually desirable to perform a “clean” shutdown.
        // To do this, the receiver first calls close, which will prevent any further messages
        // to be sent into the channel. Then, the receiver consumes the channel to completion,
        // at which point the receiver can be dropped.
        self.control.close();
        self.outgoing_session_frames.close();

        debug!("Stopped");

        outcome
    }
}

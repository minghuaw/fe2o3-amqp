//! The engine handles incoming and outgoing frames and messages to reduce
//! transferring frames/messages over channels

use std::io;
use std::time::Duration;

use fe2o3_amqp_types::definitions::{self, AmqpError};
use fe2o3_amqp_types::performatives::Close;
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;

use crate::control::ConnectionControl;
use crate::endpoint::{IncomingChannel, OutgoingChannel};
use crate::frames::amqp::{self, Frame, FrameBody};
use crate::session::frame::{SessionFrame, SessionFrameBody};
use crate::transport::Transport;
use crate::util::Running;
use crate::{endpoint, transport};

use super::{heartbeat::HeartBeat, ConnectionState};
use super::{AllocSessionError, ConnectionInnerError, ConnectionStateError, Error, OpenError};

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
    C: endpoint::Connection<State = ConnectionState> + std::fmt::Debug + Send + Sync + 'static,
    C::AllocError: Into<AllocSessionError>,
    C::CloseError: From<transport::Error>,
    C::OpenError: From<transport::Error>,
    ConnectionInnerError: From<C::Error> + From<C::OpenError> + From<C::CloseError>,
    ConnectionStateError: From<C::OpenError> + From<C::CloseError>,
    OpenError: From<C::OpenError>,
{
    async fn close_connection(
        &mut self,
        error: Option<definitions::Error>,
    ) -> Result<Running, ConnectionInnerError> {
        match self.connection.local_state() {
            ConnectionState::Start
            | ConnectionState::HeaderReceived
            | ConnectionState::HeaderSent
            | ConnectionState::HeaderExchange => Err(ConnectionInnerError::IllegalState),
            ConnectionState::OpenPipe
            | ConnectionState::OpenClosePipe
            | ConnectionState::OpenReceived
            | ConnectionState::OpenSent
            | ConnectionState::Opened => {
                self.connection
                    .send_close(&mut self.transport, error)
                    .await?;
                let (channel, close) = self.wait_for_remote_close(false).await?;
                self.connection.on_incoming_close(channel, close)?;
                Ok(Running::Stop)
            }
            ConnectionState::CloseReceived => {
                self.connection
                    .send_close(&mut self.transport, error)
                    .await?;
                Ok(Running::Stop)
            }
            ConnectionState::ClosePipe | ConnectionState::CloseSent => {
                let (channel, close) = self.wait_for_remote_close(false).await?;
                self.connection.on_incoming_close(channel, close)?;
                Ok(Running::Stop)
            }
            ConnectionState::Discarding => {
                let (channel, close) = self.wait_for_remote_close(true).await?;
                self.connection.on_incoming_close(channel, close)?;
                Ok(Running::Stop)
            }
            ConnectionState::End => Ok(Running::Stop),
        }
    }

    async fn wait_for_remote_close(
        &mut self,
        discard_other: bool,
    ) -> Result<(IncomingChannel, Close), ConnectionInnerError> {
        loop {
            let frame = self.transport.next().await.ok_or_else(|| {
                transport::Error::Io(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Expecting remote close",
                ))
            })??;

            match frame.body {
                FrameBody::Close(close) => return Ok((IncomingChannel(frame.channel), close)),
                _ => {
                    if !discard_other {
                        self.on_incoming(frame).await?;
                    }
                }
            }
        }
    }

    async fn open_inner(&mut self) -> Result<(), OpenError> {
        self.connection.send_open(&mut self.transport).await?;

        // Wait for an Open
        let frame = match self.transport.next().await {
            Some(frame) => match frame {
                Ok(fr) => fr,
                Err(error) => return Err(error.into()),
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
            FrameBody::Close(close) => match close.error {
                Some(error) => return Err(OpenError::RemoteClosedWithError(error)),
                None => return Err(OpenError::RemoteClosed),
            },
            _ => return Err(OpenError::IllegalState),
        };

        // Handle incoming remote_open
        let remote_max_frame_size = remote_open.max_frame_size.0 as usize;
        let remote_idle_timeout = remote_open.idle_time_out;
        self.connection.on_incoming_open(channel, remote_open)?;

        // update transport setting
        let local_max_frame_size = self.connection.local_open().max_frame_size.0 as usize;
        self.transport
            .set_encoder_max_frame_size(remote_max_frame_size)
            .set_decoder_max_frame_size(local_max_frame_size);

        // Set heartbeat here because in pipelined-open, the Open frame
        // may be recved after mux loop is started
        match &remote_idle_timeout {
            Some(0) | None => self.heartbeat = HeartBeat::never(),
            Some(millis) => {
                let period = Duration::from_millis(*millis as u64);
                self.heartbeat = HeartBeat::new(period);
            }
        };

        Ok(())
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

        match engine.open_inner().await {
            Ok(_) => Ok(engine),
            Err(error) => {
                match engine.close_connection(None).await {
                    Ok(_) => Err(error),
                    Err(error) => match error {
                        ConnectionInnerError::TransportError(e) => {
                            Err(OpenError::TransportError(e))
                        }
                        ConnectionInnerError::IllegalState => Err(OpenError::IllegalState),
                        ConnectionInnerError::NotImplemented(e) => {
                            Err(OpenError::NotImplemented(e))
                        }
                        ConnectionInnerError::RemoteClosed => Err(OpenError::RemoteClosed),
                        ConnectionInnerError::RemoteClosedWithError(e) => {
                            Err(OpenError::RemoteClosedWithError(e))
                        }
                        ConnectionInnerError::NotFound(_) => {
                            // This will only occur when the remote is trying to send to a session
                            // which is not supported currently
                            Err(OpenError::NotImplemented(Some(String::from(
                                "Pipelined open is not implemented",
                            ))))
                        }
                    },
                }
            }
        }
    }

    pub fn spawn(self) -> JoinHandle<Result<(), Error>> {
        tokio::spawn(self.event_loop())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn forward_to_session(
        &mut self,
        channel: IncomingChannel,
        frame: SessionFrame,
    ) -> Result<(), ConnectionInnerError> {
        match &self.connection.local_state() {
            ConnectionState::Opened => {}
            _ => return Err(ConnectionInnerError::IllegalState),
        };

        match self.connection.session_tx_by_incoming_channel(channel) {
            Some(tx) => tx.send(frame).await?,
            None => return Err(ConnectionInnerError::NotFound(None)),
        };
        Ok(())
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(name = "RECV", skip_all))]
    async fn on_incoming(&mut self, frame: Frame) -> Result<Running, ConnectionInnerError> {
        #[cfg(feature = "tracing")]
        tracing::trace!(?frame);
        #[cfg(feature = "log")]
        log::trace!("frame={:?}", frame);

        let Frame { channel, body } = frame;
        let channel = IncomingChannel(channel);
        match body {
            FrameBody::Open(open) => {
                let remote_idle_timeout = open.idle_time_out;
                self.connection.on_incoming_open(channel, open)?;

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
                self.connection.on_incoming_begin(channel, begin).await?;
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
                self.connection.on_incoming_end(channel, end).await?;
            }
            FrameBody::Close(close) => {
                let result = self.connection.on_incoming_close(channel, close);
                if matches!(
                    self.connection.local_state(),
                    ConnectionState::CloseReceived
                ) {
                    self.outgoing_session_frames.close();
                    while let Some(frame) = self.outgoing_session_frames.recv().await {
                        self.on_outgoing_session_frames(frame).await?;
                    }

                    self.connection
                        .send_close(&mut self.transport, None)
                        .await?;
                }
                result?;
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
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    async fn on_control(
        &mut self,
        control: ConnectionControl,
    ) -> Result<Running, ConnectionInnerError> {
        #[cfg(feature = "tracing")]
        tracing::debug!("{}", control);
        #[cfg(feature = "log")]
        log::debug!("{}", control);
        match control {
            ConnectionControl::Close(error) => {
                self.outgoing_session_frames.close();
                while let Some(frame) = self.outgoing_session_frames.recv().await {
                    self.on_outgoing_session_frames(frame).await?;
                }

                self.connection
                    .send_close(&mut self.transport, error)
                    .await?;
            }
            ConnectionControl::AllocateSession { tx, responder } => {
                let result = self.connection.allocate_session(tx).map_err(Into::into);
                responder
                    .send(result)
                    .map_err(|_| ConnectionInnerError::IllegalState)?;
            }
            ConnectionControl::DeallocateSession(session_id) => {
                self.connection.deallocate_session(session_id)
            }
            ConnectionControl::GetMaxFrameSize(resp) => {
                let max_frame_size = self.transport.encoder_max_frame_size();
                #[allow(unused_variables)]
                if let Err(error) = resp.send(max_frame_size) {
                    #[cfg(feature = "tracing")]
                    tracing::error!(?error);
                    #[cfg(feature = "log")]
                    log::error!("{:?}", error);
                }
            }
        }

        match self.connection.local_state() {
            ConnectionState::End => Ok(Running::Stop),
            _ => Ok(Running::Continue),
        }
    }

    #[inline]
    #[cfg_attr(feature = "tracing", tracing::instrument(name = "SEND", skip_all))]
    async fn on_outgoing_session_frames(
        &mut self,
        frame: SessionFrame,
    ) -> Result<Running, ConnectionInnerError> {
        match self.connection.local_state() {
            ConnectionState::Opened => {}
            _ => return Err(ConnectionInnerError::IllegalState),
        }

        let SessionFrame { channel, body } = frame;
        let channel = OutgoingChannel(channel);
        let frame = match body {
            SessionFrameBody::Begin(begin) => self.connection.on_outgoing_begin(channel, begin)?,
            SessionFrameBody::Attach(attach) => Frame::new(channel, FrameBody::Attach(attach)),
            SessionFrameBody::Flow(flow) => Frame::new(channel, FrameBody::Flow(flow)),
            SessionFrameBody::Transfer {
                performative,
                payload,
            } => Frame::new(
                channel,
                FrameBody::Transfer {
                    performative,
                    payload,
                },
            ),
            SessionFrameBody::Disposition(disposition) => {
                Frame::new(channel, FrameBody::Disposition(disposition))
            }
            SessionFrameBody::Detach(detach) => Frame::new(channel, FrameBody::Detach(detach)),
            SessionFrameBody::End(end) => self.connection.on_outgoing_end(channel, end)?,
        };

        #[cfg(feature = "tracing")]
        tracing::trace!(channel = frame.channel, frame = ?frame.body);
        #[cfg(feature = "log")]
        log::trace!("channel = {}, frame = {:?}", frame.channel, frame.body);
        self.transport.send(frame).await?;
        Ok(Running::Continue)
    }

    #[inline]
    async fn on_heartbeat(&mut self) -> Result<Running, ConnectionInnerError> {
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
    async fn on_error(
        &mut self,
        error: &ConnectionInnerError,
    ) -> Result<Running, ConnectionInnerError> {
        match error {
            ConnectionInnerError::TransportError(_) => Ok(Running::Stop),
            ConnectionInnerError::IllegalState => {
                let error = definitions::Error::new(AmqpError::IllegalState, None, None);
                self.close_connection(Some(error)).await?;
                Ok(Running::Stop)
            }
            ConnectionInnerError::NotImplemented(description) => {
                let error =
                    definitions::Error::new(AmqpError::NotImplemented, description.clone(), None);
                self.close_connection(Some(error)).await?;
                Ok(Running::Stop)
            }
            ConnectionInnerError::NotFound(description) => {
                let error = definitions::Error::new(AmqpError::NotFound, description.clone(), None);
                self.close_connection(Some(error)).await?;
                Ok(Running::Stop)
            }
            ConnectionInnerError::RemoteClosed | ConnectionInnerError::RemoteClosedWithError(_) => {
                self.close_connection(None).await
            }
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(name = "Connection::event_loop", skip(self), fields(container_id = %self.connection.local_open().container_id)))]
    async fn event_loop(mut self) -> Result<(), Error> {
        let mut outcome = Ok(());
        loop {
            let result = tokio::select! {
                _ = self.heartbeat.next() => self.on_heartbeat().await,
                incoming = self.transport.next() => {
                    let result = match incoming {
                        Some(incoming) => {
                            match incoming {
                                Ok(frame) => self.on_incoming(frame).await,
                                Err(err) => Err(err.into()),
                            }
                        },
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
                                | ConnectionState::CloseSent => Err(ConnectionInnerError::IllegalState),
                                ConnectionState::ClosePipe
                                | ConnectionState::Discarding
                                | ConnectionState::End => Ok(Running::Stop),
                            }
                        },
                    };

                    result
                },
                control = self.control.recv() => {
                    match control {
                        Some(control) => self.on_control(control).await,
                        None => {
                            // All control channel are dropped (which is impossible)
                            // Finish closing the connection
                            loop {
                                match self.connection.local_state() {
                                    ConnectionState::Opened
                                    | ConnectionState::CloseReceived
                                    | ConnectionState::OpenSent
                                    | ConnectionState::OpenPipe => {
                                        if let Err(_err) = self.on_control(ConnectionControl::Close(None)).await {
                                            #[cfg(feature = "tracing")]
                                            tracing::error!("err {:?}", _err);
                                            #[cfg(feature = "log")]
                                            log::error!("err {:?}", _err);
                                        }
                                    }
                                    ConnectionState::Discarding
                                    | ConnectionState::CloseSent
                                    | ConnectionState::OpenClosePipe
                                    | ConnectionState::ClosePipe => {
                                        match self.transport.next().await {
                                            Some(Ok(frame)) => {
                                                let _ = self.on_incoming(frame).await;
                                            },
                                            Some(Err(_err)) => {
                                                #[cfg(feature = "tracing")]
                                                tracing::error!("err {:?}", _err);
                                                #[cfg(feature = "log")]
                                                log::error!("err {:?}", _err);
                                            },
                                            None => break
                                        }
                                    },
                                    ConnectionState::End => break,
                                    _ => break, // This should not happen
                                }
                            }

                            Ok(Running::Stop)
                        }
                    }
                },
                frame = self.outgoing_session_frames.recv() => {
                    match frame {
                        Some(frame) => self.on_outgoing_session_frames(frame).await,
                        None => {
                            // Upon closing, the outgoing_session_frames channel will be closed
                            // first while the connection may still be waiting for remote
                            // close frame.
                            Ok(Running::Continue)
                        }
                    }
                }
            };

            let running = match result {
                Ok(running) => running,
                Err(error) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!("{:?}", error);
                    #[cfg(feature = "log")]
                    log::error!("{:?}", error);
                    // let running = self.on_error(&error).await;
                    match self.on_error(&error).await {
                        Ok(running) => {
                            outcome = Err(error);
                            running
                        }
                        Err(error) => {
                            // Stop the session if error cannot be handled
                            #[cfg(feature = "tracing")]
                            tracing::error!("Unable to handle error {:?}", error);
                            #[cfg(feature = "log")]
                            log::error!("Unable to handle error {:?}", error);
                            outcome = Err(error);
                            Running::Stop
                        }
                    }
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
        let close = self.transport.close().await.map_err(Into::into);

        #[cfg(feature = "tracing")]
        tracing::debug!("Stopped");
        #[cfg(feature = "log")]
        log::debug!("Stopped");

        outcome.and(close).map_err(Into::into)
    }
}

//! WebSocket in wasm with web-sys

use std::io;

use futures_util::{Sink, Stream};
use pin_project_lite::pin_project;
use tokio::sync::mpsc::UnboundedReceiver;
use tungstenite::protocol::CloseFrame;
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

use crate::Error;

enum ReadyState {
    Connecting,
    Open,
    Closing,
    Closed,
}

impl TryFrom<u16> for ReadyState {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ReadyState::Connecting),
            1 => Ok(ReadyState::Open),
            2 => Ok(ReadyState::Closing),
            3 => Ok(ReadyState::Closed),
            _ => Err(()),
        }
    }
}

pin_project! {
    /// A wrapper over [`web_sys::WebSocket`] that implements `Stream` and `Sink`.
    #[derive(Debug)]
    pub struct WasmWebSocketStream {
        inner: WebSocket,
        #[pin]
        receiver: UnboundedReceiver<Result<tungstenite::Message, Error>>,
        _on_message_callback: Closure<dyn FnMut(MessageEvent)>,
        _on_close_callback: Closure<dyn FnMut(CloseEvent)>,
        _on_error_callback: Closure<dyn FnMut(ErrorEvent)>,
    }
}

impl From<WasmWebSocketStream> for super::WebSocketStream<WasmWebSocketStream> {
    fn from(stream: WasmWebSocketStream) -> Self {
        super::WebSocketStream {
            inner: stream,
            current_binary: None,
        }
    }
}

impl WasmWebSocketStream {
    /// Connect websocket
    pub async fn connect(addr: impl AsRef<str>) -> Result<Self, Error> {
        let ws = WebSocket::new_with_str(addr.as_ref(), super::SEC_WEBSOCKET_PROTOCOL_AMQP)
            .map_err(|_| {
                Error::Tungstenite(tungstenite::Error::Io(io::Error::other(
                    "Failed to create WebSocket with web-sys",
                )))
            })?;

        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let (open_tx, open_rx) = tokio::sync::oneshot::channel();
        let on_open_callback = Closure::once_into_js(move || {
            let _ = open_tx.send(());
        });
        ws.set_onopen(Some(on_open_callback.as_ref().unchecked_ref()));

        let (err_tx, err_rx) = tokio::sync::oneshot::channel();
        let on_error_callback = Closure::once_into_js(move |event: ErrorEvent| {
            let _ = err_tx.send(event);
        });
        ws.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));

        let result = tokio::select! {
            _ = open_rx => Ok(()),
            event = err_rx => Err(Error::Tungstenite(tungstenite::Error::Io(io::Error::other(format!("Failed to connect to WebSocket: {:?}", event))))),
        };
        ws.set_onopen(None);
        ws.set_onerror(None);
        result?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let tx_clone = tx.clone();
        let on_message_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
            let result = message_from_event(event);
            let _ = tx_clone.send(result);
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(on_message_callback.as_ref().unchecked_ref()));

        let tx_clone = tx.clone();
        let on_error_callback = Closure::wrap(Box::new(move |event: ErrorEvent| {
            let _ = tx_clone.send(Err(Error::Tungstenite(tungstenite::Error::Io(io::Error::other(format!(
                "WebSocket error: {:?}",
                event
            ))))));
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));

        let on_close_callback = Closure::wrap(Box::new(move |event: CloseEvent| {
            let message = tungstenite::Message::Close(Some(CloseFrame {
                code: event.code().into(),
                reason: event.reason().into(),
            }));
            let _ = tx.send(Ok(message));
        }) as Box<dyn FnMut(CloseEvent)>);
        ws.set_onclose(Some(on_close_callback.as_ref().unchecked_ref()));

        Ok(Self {
            inner: ws,
            receiver: rx,
            _on_message_callback: on_message_callback,
            _on_close_callback: on_close_callback,
            _on_error_callback: on_error_callback,
        })
    }
}

impl super::WebSocketStream<WasmWebSocketStream> {
    /// Connect websocket over wasm
    pub async fn connect(addr: impl AsRef<str>) -> Result<Self, Error> {
        let stream = WasmWebSocketStream::connect(addr).await?;
        Ok(Self::from(stream))
    }
}

fn message_from_event(event: MessageEvent) -> Result<tungstenite::Message, Error> {
    let data = event.data();
    let data = data.dyn_into::<js_sys::ArrayBuffer>().map_err(|_| {
        Error::Tungstenite(tungstenite::Error::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            "Only Binary data is supported",
        )))
    })?;
    let data = js_sys::Uint8Array::new(&data);
    let data = data.to_vec();
    Ok(tungstenite::Message::binary(data))
}

impl Stream for WasmWebSocketStream {
    type Item = Result<tungstenite::Message, Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut this = self.project();
        this.receiver.poll_recv(cx)
    }
}

impl Sink<tungstenite::Message> for WasmWebSocketStream {
    type Error = Error;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        match self.inner.ready_state().try_into() {
            Ok(ReadyState::Open) => std::task::Poll::Ready(Ok(())),
            _ => std::task::Poll::Ready(Err(Error::Tungstenite(tungstenite::Error::ConnectionClosed))),
        }
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: tungstenite::Message) -> Result<(), Self::Error> {
        match self.inner.ready_state().try_into() {
            Ok(ReadyState::Open) => match item {
                tungstenite::Message::Text(item) => self
                    .inner
                    .send_with_str(&item)
                    .map_err(|_| Error::Tungstenite(tungstenite::Error::ConnectionClosed)),
                tungstenite::Message::Binary(item) => self
                    .inner
                    .send_with_u8_array(&item)
                    .map_err(|_| Error::Tungstenite(tungstenite::Error::ConnectionClosed)),
                tungstenite::Message::Close(frame) => match frame {
                    Some(frame) => self
                        .inner
                        .close_with_code_and_reason(frame.code.into(), &frame.reason)
                        .map_err(|_| Error::Tungstenite(tungstenite::Error::ConnectionClosed)),
                    None => self
                        .inner
                        .close()
                        .map_err(|_| Error::Tungstenite(tungstenite::Error::ConnectionClosed)),
                },
                tungstenite::Message::Ping(_)
                | tungstenite::Message::Pong(_)
                | tungstenite::Message::Frame(_) => Err(Error::Tungstenite(tungstenite::Error::Io(io::Error::other(
                    "Sending Ping, Pong and Frame is not supported",
                )))),
            },
            _ => Err(Error::Tungstenite(tungstenite::Error::ConnectionClosed)),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner
            .close()
            .map_err(|_| Error::Tungstenite(tungstenite::Error::AlreadyClosed))?;
        std::task::Poll::Ready(Ok(()))
    }
}

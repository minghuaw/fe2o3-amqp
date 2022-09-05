use std::{
    io::{Cursor, Read},
    task::Poll,
};

use bytes::{BytesMut};
use futures_util::{ready, Sink, Stream};
use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite};

pin_project! {
    #[derive(Debug)]
    pub(crate) struct ReadWriteAdaptor<R, W> {
        #[pin]
        framed_read: FramedRead<R, BytesCodec>,
        #[pin]
        framed_write: FramedWrite<W, BytesCodec>,
        read_buffer: Option<Cursor<Vec<u8>>>,
    }
}

impl<R, W> ReadWriteAdaptor<R, W> {
    pub fn new(
        framed_read: FramedRead<R, BytesCodec>,
        framed_write: FramedWrite<W, BytesCodec>,
    ) -> Self {
        Self {
            framed_read,
            framed_write,
            read_buffer: None,
        }
    }
}

impl<R, W> AsyncRead for ReadWriteAdaptor<R, W>
where
    R: AsyncRead + Unpin,
{
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.project();
        let mut framed_read = this.framed_read;

        let (item_to_copy, len_to_read) = loop {
            if let Some(cursor) = this.read_buffer {
                let len = cursor.get_ref().len() as u64;
                let pos = cursor.position();
                if pos < len {
                    break (cursor, len - pos);
                }
            }

            let msg = match ready!(framed_read.as_mut().poll_next(cx)) {
                Some(Ok(msg)) => msg,
                Some(Err(err)) => return Poll::Ready(Err(err)),
                None => return Poll::Ready(Ok(())), // EOF
            };

            *this.read_buffer = Some(Cursor::new(msg.into()))
        };

        // Copy it!
        let len_to_read = buf
            .remaining()
            .min(len_to_read.min(usize::MAX as u64) as usize);
        let unfilled_buf = buf.initialize_unfilled_to(len_to_read);
        let len = item_to_copy.read(unfilled_buf)?;
        buf.advance(len);
        Poll::Ready(Ok(()))
    }
}

impl<R, W> AsyncWrite for ReadWriteAdaptor<R, W>
where
    W: AsyncWrite + Unpin,
{
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let mut this = self.project();
        // ready!(this.framed_write.as_mut().poll_ready(cx))?;
        match <FramedWrite<W, BytesCodec> as futures_util::Sink<BytesMut>>::poll_ready(
            this.framed_write.as_mut(),
            cx,
        ) {
            Poll::Ready(result) => match result {
                Ok(_) => {},
                Err(err) => return Poll::Ready(Err(err)),
            },
            Poll::Pending => return Poll::Pending,
        }
        let n = buf.len();
        let item = BytesMut::from(buf);
        match this.framed_write.start_send(item) {
            Ok(_) => Poll::Ready(Ok(n)),
            Err(error) => Poll::Ready(Err(error)),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let this = self.project();
        <FramedWrite<W, BytesCodec> as futures_util::Sink<BytesMut>>::poll_flush(this.framed_write, cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let this = self.project();
        <FramedWrite<W, BytesCodec> as futures_util::Sink<BytesMut>>::poll_close(this.framed_write, cx)
    }
}

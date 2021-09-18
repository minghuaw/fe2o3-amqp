use std::time::Duration;

use futures_util::Stream;
use pin_project_lite::pin_project;

use tokio::{sync::mpsc::{Receiver, Sender}, time::Sleep};

use super::mux::MuxControl;

pin_project! {
    pub struct HeartBeat {
        remote_idle_timeout: Duration,
        #[pin]
        delay: Sleep,
    }
}

// pin_project! {
//     /// Similar to `tokio::time::Timeout` but resets the delay for every success stream.
//     #[must_use = "futures do nothing unless you `.await` or poll them"]
//     #[derive(Debug)]
//     pub struct IdleTimeout<T: Stream> {
//         #[pin]
//         stream: T,
//         #[pin]
//         delay: Sleep,
//     }
// }


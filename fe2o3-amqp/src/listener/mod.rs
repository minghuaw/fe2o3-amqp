//! Listeners

use std::{io, net::SocketAddr};

use async_trait::async_trait;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
};

pub mod connection;
pub mod link;
pub mod session;
pub mod builder;
pub mod sasl_acceptor;

pub use self::connection::*;

// /// Trait for listeners
// #[async_trait]
// pub trait Listener {
//     /// Type of accepted IO stream
//     type Stream: AsyncRead + AsyncWrite + std::fmt::Debug + Send + Unpin + 'static;
//     // type Stream;

//     /// Type for local addr
//     type Addr;

//     /// Obtain the local address
//     fn local_addr(&self) -> Result<Self::Addr, io::Error>;

//     /// Accept an incoming stream
//     async fn accept(&self) -> Result<Self::Stream, io::Error>;
// }

// #[async_trait]
// impl Listener for TcpListener {
//     type Stream = TcpStream;
//     type Addr = SocketAddr;

//     fn local_addr(&self) -> Result<Self::Addr, io::Error> {
//         self.local_addr()
//     }

//     async fn accept(&self) -> Result<Self::Stream, io::Error> {
//         self.accept().await.map(|(socket, _)| socket)
//     }
// }

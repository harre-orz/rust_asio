// Copyright 2016 Haruhiko Uchida
//
// The software is released under the MIT license. see LICENSE.txt
// https://github.com/harre-orz/rust_asio/blob/master/LICENSE.txt

//! The asyncio is Asynchronous Input/Output library.
//!
//! # Usage
//! This crate is on [github](https://github.com/harre-orz/rust_asio "github") and can be used by adding asyncio to the dependencies in your project's Cargo.toml.
//!
//! ```toml
//! [dependencies]
//! rust_asio = "*"
//! ```
//!
//! And this in your crate root:
//!
//! ```
//! extern crate asyncio;
//! ```
//!
//! For example, TCP connection code:
//!
//! ```
//! use std::io;
//! use std::sync::Arc;
//! use asyncio::*;
//! use asyncio::ip::*;
//! use asyncio::socket_base::*;
//!
//! fn on_accept(sv: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
//!   match res {
//!     Ok((soc, ep)) => { /* do something */ },
//!     Err(err) => panic!("{}", err),
//!   }
//! }
//!
//! fn on_connect(cl: Arc<TcpSocket>, res: io::Result<()>) {
//!   match res {
//!     Ok(_) => { /* do something */ },
//!     Err(err) => panic!("{}", err),
//!   }
//! }
//!
//! fn main() {
//!   let io = &IoService::new();
//!
//!   let sv = Arc::new(TcpListener::new(io, Tcp::v4()).unwrap());
//!   sv.set_option(ReuseAddr::new(true)).unwrap();
//!   let ep = TcpEndpoint::new(IpAddrV4::any(), 12345);
//!   sv.bind(&ep).unwrap();
//!   sv.listen().unwrap();
//!   sv.async_accept(bind(on_accept, &sv));
//!
//!   let cl = Arc::new(TcpSocket::new(io, Tcp::v4()).unwrap());
//!   cl.async_connect(&ep, bind(on_connect, &cl));
//!
//!   io.run();
//! }
//! ```

#![feature(fnbox, test)]

extern crate test;
extern crate libc;
extern crate time;
extern crate context;

use std::mem;
use std::sync::Arc;

mod unsafe_cell;

mod async_result;
pub use self::async_result::Handler;

mod backbone;
use backbone::{SHUT_RD, SHUT_WR, SHUT_RDWR, RawFd, AsRawFd, sockaddr};

/// Possible values which can be passed to the shutdown method.
pub enum Shutdown {
    /// Indicates that the reading portion of this socket should be shut down.
    Read = SHUT_RD as isize,

    /// Indicates that the writing portion of this socket should be shut down.
    Write = SHUT_WR as isize,

    /// Shut down both the reading and writing portions of this socket.
    Both = SHUT_RDWR as isize,
}

pub trait SockAddr : Clone + Send + 'static {
    fn as_sockaddr(&self) -> &sockaddr;

    fn as_mut_sockaddr(&mut self) -> &mut sockaddr;

    fn capacity(&self) -> usize;

    fn size(&self) -> usize;

    unsafe fn resize(&mut self, size: usize);
}

pub trait Endpoint<P> : SockAddr {
    fn protocol(&self) -> P;
}

pub trait Protocol : Clone + Send + 'static {
    type Endpoint : Endpoint<Self>;

    /// Reurns a value suitable for passing as the domain argument.
    fn family_type(&self) -> i32;

    /// Returns a value suitable for passing as the type argument.
    fn socket_type(&self) -> i32;

    /// Returns a value suitable for passing as the protocol argument.
    fn protocol_type(&self) -> i32;

    unsafe fn uninitialized(&self) -> Self::Endpoint;
}

pub trait IoControl {
    type Data;

    fn name(&self) -> i32;

    fn data(&mut self) -> &mut Self::Data;
}

pub trait SocketOption<P: Protocol> {
    type Data;

    fn level(&self, pro: &P) -> i32;

    fn name(&self, pro: &P) -> i32;
}

pub trait GetSocketOption<P: Protocol> : SocketOption<P> + Default {
    fn data_mut(&mut self) -> &mut Self::Data;

    fn resize(&mut self, _size: usize) {
    }
}

pub trait SetSocketOption<P: Protocol> : SocketOption<P> {
    fn data(&self) -> &Self::Data;

    fn size(&self)  -> usize {
        mem::size_of::<Self::Data>()
    }
}

/// Traits to the associated with `IoService`.
pub trait IoObject : Sized {
    /// Returns a `IoService` associated with this object.
    fn io_service(&self) -> &IoService;
}

#[doc(hidden)]
pub trait FromRawFd<P: Protocol> : AsRawFd + Send + 'static {
    unsafe fn from_raw_fd<T: IoObject>(io: &T, pro: P, fd: RawFd) -> Self;
}

mod io_service;
use io_service::IoServiceBase;
pub use io_service::{IoServiceWork};

#[derive(Clone)]
pub struct IoService(Arc<IoServiceBase>);

mod handlers;
pub use self::handlers::{ArcHandler, bind, Strand, StrandHandler, Coroutine, CoroutineHandler, spawn};

mod stream;
pub use self::stream::*;

mod stream_socket;
pub use self::stream_socket::StreamSocket;

mod dgram_socket;
pub use self::dgram_socket::DgramSocket;

mod raw_socket;
pub use self::raw_socket::RawSocket;

mod seq_packet_socket;
pub use self::seq_packet_socket::SeqPacketSocket;

mod socket_listener;
pub use self::socket_listener::SocketListener;

pub mod clock;
pub type SystemTimer = clock::WaitTimer<clock::SystemClock>;
pub type SteadyTimer = clock::WaitTimer<clock::SteadyClock>;

#[cfg(all(not(feature = "asio_no_signal_set"), target_os = "linux"))]
mod signal_set;

#[cfg(all(not(feature = "asio_no_signal_set"), target_os = "linux"))]
pub use self::signal_set::{Signal, SignalSet, raise};

pub mod socket_base;

pub mod ip;

pub mod local;

pub mod generic;

pub mod posix;

mod from_str;

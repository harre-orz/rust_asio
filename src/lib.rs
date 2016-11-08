// Copyright 2016 Haruhiko Uchida
//
// The software is released under the MIT license. see LICENSE.txt
// https://github.com/harre-orz/rust_asio/blob/master/LICENSE.txt

//! The asyncio is Asynchronous Input/Output library, that made based on [boost::asio](http://www.boost.org/doc/libs/1_62_0/doc/html/boost_asio.html) c++ library.
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
//! For example, TCP asynchronous connection code:
//!
//! ```
//! use asyncio::*;
//! use asyncio::ip::*;
//! use asyncio::socket_base::*;
//! use std::io;
//! use std::sync::Arc;
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
//!   let ep = TcpEndpoint::new(Tcp::v4(), 12345);
//!   let sv = Arc::new(TcpListener::new(io, ep.protocol()).unwrap());
//!   sv.set_option(ReuseAddr::new(true)).unwrap();
//!   sv.bind(&ep).unwrap();
//!   sv.listen().unwrap();
//!   sv.async_accept(wrap(on_accept, &sv));
//!
//!   let cl = Arc::new(TcpSocket::new(io, ep.protocol()).unwrap());
//!   cl.async_connect(&ep, wrap(on_connect, &cl));
//!
//!   io.run();
//! }
//! ```
//!
//! For example, TCP connection with coroutine code:
//!
//! ```
//! use asyncio::*;
//! use asyncio::ip::*;
//! use asyncio::socket_base::*;
//!
//! fn main() {
//!   let io = &IoService::new();
//!
//!   let ep = TcpEndpoint::new(Tcp::v4(), 12345);
//!   let sv = TcpListener::new(io, ep.protocol()).unwrap();
//!   sv.set_option(ReuseAddr::new(true)).unwrap();
//!   sv.bind(&ep).unwrap();
//!   sv.listen().unwrap();
//!
//!   IoService::spawn(io, move |co| {
//!     let (soc, ep): (TcpSocket, TcpEndpoint) = sv.async_accept(co.wrap()).unwrap();
//!     /* do something */
//!   });
//!
//!   IoService::spawn(io, move |co| {
//!     let cl = TcpSocket::new(co.io_service(), ep.protocol()).unwrap();
//!     cl.async_connect(&ep, co.wrap()).unwrap();
//!     /* do something */
//!   });
//!
//!   io.run();
//! }
//! ```
//!

#![feature(fnbox, test)]

#[macro_use]
extern crate lazy_static;
extern crate libc;
extern crate errno;
extern crate thread_id;
extern crate test;
#[cfg(feature = "context")] extern crate context;
#[cfg(feature = "termios")] extern crate termios;

//------
// Core system

macro_rules! libc_try {
    ($expr:expr) => (
        match unsafe { $expr } {
        rc if rc >= 0 => rc,
        _ => return Err(::std::io::Error::last_os_error()),
    })
}

macro_rules! libc_unwrap {
    ($expr:expr) => (
        match unsafe { $expr } {
        rc if rc >= 0 => rc,
        _ => panic!("{}", ::std::io::Error::last_os_error()),
    })
}

macro_rules! libc_ign {
    ($expr:expr) => (
        let _err = unsafe { $expr };
        debug_assert!(_err >= 0);
    )
}

mod unsafe_cell;

mod error;

mod traits;
pub use self::traits::*;

pub mod clock;

mod io_service;
pub use self::io_service::{IoObject, FromRawFd, IoService, IoServiceWork, Handler, Strand, StrandImmutable, wrap};
#[cfg(feature = "context")] pub use self::io_service::Coroutine;

//---------
// Sockets

/// Socket address operations
mod sa_ops;

/// File descriptor operations
mod fd_ops;

mod streambuf;
pub use self::streambuf::StreamBuf;

mod stream;
pub use self::stream::{Stream, MatchCondition, read_until, write_until, async_read_until, async_write_until};

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

pub mod socket_base;

pub mod ip;

pub mod generic;

mod from_str;

//--------
// Timers

mod waitable_timer;
pub use self::waitable_timer::{WaitableTimer, SteadyTimer, SystemTimer};

//-----
// SSL

//-------------
// Serial port

pub mod serial_port;

//----------------
// Signal handing

#[cfg(target_os = "linux")] mod signal_set;
#[cfg(target_os = "linux")] pub use self::signal_set::{Signal, SignalSet, raise};

//-----------------------
// Posix specific

#[cfg(unix)]
pub mod local;

#[cfg(unix)]
pub mod posix;

//------------------
// Windows specific

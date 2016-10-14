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
//!   sv.async_accept(wrap(on_accept, &sv));
//!
//!   let cl = Arc::new(TcpSocket::new(io, Tcp::v4()).unwrap());
//!   cl.async_connect(&ep, wrap(on_connect, &cl));
//!
//!   io.run();
//! }
//! ```

#![feature(fnbox, test)]

extern crate test;
extern crate libc;
extern crate time;
extern crate thread_id;
#[cfg(feature = "context")] extern crate context;

macro_rules! libc_try {
    ($expr:expr) => (match unsafe { $expr } {
        rc if rc >= 0 => rc,
        _ => return Err(::std::io::Error::last_os_error()),
    })
}

macro_rules! libc_ign {
    ($expr:expr) => (let _ = unsafe { $expr };)
}

mod error_code;

mod unsafe_cell;

mod io_service;
pub use self::io_service::{IoObject, IoService, IoServiceWork, Strand, wrap};
#[cfg(feature = "context")] pub use self::io_service::{Coroutine};

mod traits;
pub use self::traits::*;

mod async_result;
pub use self::async_result::Handler;

mod backbone;

mod buffer;
pub use self::buffer::StreamBuf;

mod stream;
pub use self::stream::{MatchCondition, Stream, read_until, write_until, async_read_until, async_write_until};

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
pub type SystemTimer = clock::WaitableTimer<clock::SystemClock>;
pub type SteadyTimer = clock::WaitableTimer<clock::SteadyClock>;

#[cfg(all(not(feature = "asyncio_no_signal_set"), target_os = "linux"))]
mod signal_set;

#[cfg(all(not(feature = "asyncio_no_signal_set"), target_os = "linux"))]
pub use self::signal_set::{Signal, SignalSet, raise};

pub mod socket_base;

pub mod ip;

pub mod local;

pub mod generic;

pub mod posix;

mod from_str;

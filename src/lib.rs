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
//!
//! # Warnings
//! If use asynchronous function, `MUST` be wrapping in `Arc`, `Strand` or `Coroutine`.
//!
//! ## Examples
//! ```
//! use asyncio::*;
//! use asyncio::ip::*;
//! use std::io;
//! use std::sync::Arc;
//!
//! fn good_example(soc: Arc<TcpListener>) {
//!   soc.async_accept(wrap(|soc: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>| {
//!     // OK
//!   }, &soc));
//! }
//!
//! fn bad_example(soc: TcpListener, dummy: Arc<TcpListener>) {
//!   soc.async_accept(wrap(|soc: Arc<TcpListener>, res: io::Result<(TcpSocket, TcpEndpoint)>| {
//!     // Segmentation fault
//!   }, &dummy));
//! }
//! ```
//!
//! ## Examples
//! ```
//! use asyncio::*;
//! use asyncio::ip::*;
//! use std::io;
//! use std::sync::Arc;
//!
//! struct Client {
//!   soc: TcpSocket,
//!   buf: [u8; 256],
//! }
//!
//! fn good_example(mut cl: Strand<Client>) {
//!   let buf = unsafe { &mut cl.get().buf };
//!
//!   cl.soc.async_read_some(buf, cl.wrap(|cl: Strand<Client>, res: io::Result<usize>| {
//!     // OK
//!   }));
//!
//!   cl.soc.async_read_some(buf, cl.wrap(|cl: Strand<Client>, res: io::Result<usize>| {
//!     // OK
//!   }));
//! }
//!
//! unsafe impl IoObject for Client {
//!   fn io_service(&self) -> &IoService { self.soc.io_service() }
//! }
//!
//! fn bad_example(mut cl: Arc<Client>) {
//!   use std::slice;
//!   let buf = unsafe { slice::from_raw_parts_mut(cl.buf.as_ptr() as *mut _, cl.buf.len()) };
//!
//!   cl.soc.async_read_some(buf, wrap(|cl: Arc<Client>, res: io::Result<usize>| {
//!     // Occurred data race for buf
//!   }, &cl));
//!
//!   cl.soc.async_read_some(buf, wrap(|cl: Arc<Client>, res: io::Result<usize>| {
//!     // Occurred data race for buf
//!   }, &cl));
//! }
//! ```

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
pub use self::io_service::{IoObject, FromRawFd, IoService, IoServiceWork, Handler, Strand, wrap};
#[cfg(feature = "context")] pub use self::io_service::{Coroutine, spawn};

//---------
// Sockets

/// Socket address operations
mod sa_ops;

/// File descriptor operations
mod fd_ops;

mod buffer;
pub use self::buffer::{StreamBuf, MatchCondition};

mod stream;
pub use self::stream::{Stream, read_until, write_until, async_read_until, async_write_until};

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
pub use self::waitable_timer::WaitableTimer;
pub type SystemTimer = WaitableTimer<clock::SystemClock>;
pub type SteadyTimer = WaitableTimer<clock::SteadyClock>;

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

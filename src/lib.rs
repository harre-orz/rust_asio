// Copyright 2017 Haruhiko Uchida
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
//!
//! use std::io;
//! use std::sync::{Arc, Mutex};
//!
//! fn on_accept(sv: Arc<Mutex<TcpListener>>, res: io::Result<(TcpSocket, TcpEndpoint)>) {
//!   match res {
//!     Ok((soc, ep)) => { /* do something */ },
//!     Err(err) => panic!("{}", err),
//!   }
//! }
//!
//! fn on_connect(cl: Arc<Mutex<TcpSocket>>, res: io::Result<()>) {
//!   match res {
//!     Ok(_) => { /* do something */ },
//!     Err(err) => panic!("{}", err),
//!   }
//! }
//!
//! fn main() {
//!   let ctx = &IoContext::new().unwrap();
//!
//!   let ep = TcpEndpoint::new(IpAddrV4::loopback(), 12345);
//!   let sv = TcpListener::new(ctx, ep.protocol()).unwrap();
//!   sv.set_option(ReuseAddr::new(true)).unwrap();
//!   sv.bind(&ep).unwrap();
//!   sv.listen().unwrap();
//!   let sv = Arc::new(Mutex::new(sv));
//!   sv.lock().unwrap().async_accept(wrap(on_accept, &sv));
//!
//!   let cl = Arc::new(Mutex::new(TcpSocket::new(ctx, ep.protocol()).unwrap()));
//!   cl.lock().unwrap().async_connect(&ep, wrap(on_connect, &cl));
//!
//!   ctx.run();
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
//!   let ctx = &IoContext::new().unwrap();
//!
//!   let ep = TcpEndpoint::new(IpAddrV4::loopback(), 12345);
//!   let mut sv = TcpListener::new(ctx, ep.protocol()).unwrap();
//!   sv.set_option(ReuseAddr::new(true)).unwrap();
//!   sv.bind(&ep).unwrap();
//!   sv.listen().unwrap();
//!
//!   IoContext::spawn(ctx, move |co| {
//!     let (soc, ep) = sv.async_accept(co.wrap()).unwrap();
//!     /* do something */
//!   });
//!
//!   IoContext::spawn(ctx, move |co| {
//!     let mut cl = TcpSocket::new(co.as_ctx(), ep.protocol()).unwrap();
//!     cl.async_connect(&ep, co.wrap()).unwrap();
//!     /* do something */
//!   });
//!
//!   ctx.run();
//! }
//! ```
//!

#![allow(dead_code)]

#[macro_use] extern crate bitflags;
#[macro_use] extern crate lazy_static;
extern crate kernel32;
extern crate winapi;
extern crate libc;
extern crate ws2_32;
extern crate errno;
#[cfg(feature = "context")] extern crate context;
#[cfg(feature = "termios")] extern crate termios;
#[cfg(feature = "openssl")] extern crate openssl;
#[cfg(feature = "openssl-sys")] extern crate openssl_sys;
#[cfg(feature = "test")] extern crate test;

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
        }
    )
}

#[cfg(debug_assertions)]
macro_rules! libc_ign {
    ($expr:expr) => (
        if unsafe { $expr } < 0 {
            panic!("{}", ::std::io::Error::last_os_error());
        }
    )
}

#[cfg(not(debug_assertions))]
macro_rules! libc_ign {
    ($expr:expr) => (
        let _ = unsafe { $expr };
    )
}

mod unsafe_cell;

mod prelude;
pub use self::prelude::*;

mod ffi;
pub use self::ffi::{RawFd, AsRawFd};

mod error;

pub mod socket_base;

mod buffers;
pub use self::buffers::StreamBuf;

mod core;
pub use self::core::{IoContext, AsIoContext, IoContextWork, Socket};

mod async;
pub use self::async::{Handler, Strand, StrandImmutable, wrap};
#[cfg(feature = "context")] pub use self::async::Coroutine;

mod reactive_io;

mod streams;
pub use self::streams::{Stream, MatchCondition,
                        //read_until, write_until, async_read_until, async_write_until
};

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

pub mod ip;

mod from_str;

#[cfg(unix)] pub mod local;

#[cfg(unix)] pub mod posix;

pub mod generic;

#[cfg(feature = "termios")] pub mod serial_port;

//#[cfg(feature = "openssl")] pub mod ssl;

#[cfg(target_os = "linux")] mod signal_set;
#[cfg(target_os = "linux")] pub use self::signal_set::{Signal, SignalSet, raise};

pub mod clock;

mod waitable_timer;
pub use self::waitable_timer::{WaitableTimer, SteadyTimer, SystemTimer};

// asyncio
//
// The software is released under the MIT license. see LICENSE.txt
// https://github.com/harre-orz/rust_asio/blob/master/LICENSE.txt

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

pub mod prelude;
pub use self::prelude::*;

pub mod ffi;

pub mod core;
pub use self::core::{IoContext, AsIoContext, IoContextWork};

pub mod async;
pub use self::async::{Handler, Strand, StrandImmutable, wrap};
pub use self::async::{Coroutine, spawn};

pub mod streams;
pub use self::streams::{Stream, StreamBuf, MatchCond};

pub mod socket_base;

pub mod dgram_socket;
pub use self::dgram_socket::DgramSocket;

pub mod stream_socket;
pub use self::stream_socket::StreamSocket;

pub mod socket_listener;
pub use self::socket_listener::SocketListener;

pub mod generic;

pub mod local;

pub mod ip;

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

mod prelude;
pub use self::prelude::*;

mod ffi;

mod core;
pub use self::core::{IoContext, AsIoContext, IoContextWork};

mod async;
pub use self::async::{Handler, Strand, StrandImmutable, wrap};

mod buffers;
pub use self::buffers::*;

mod streams;
pub use self::streams::Stream;

pub mod socket_base;

mod dgram_socket;
pub use self::dgram_socket::DgramSocket;

mod stream_socket;
pub use self::stream_socket::StreamSocket;

mod socket_listener;
pub use self::socket_listener::SocketListener;

pub mod generic;

pub mod local;

pub mod ip;

// asyncio
//
// The software is released under the MIT license. see LICENSE.txt
// https://github.com/harre-orz/rust_asio/blob/master/LICENSE.txt

// #[macro_use] extern crate bitflags;
// #[macro_use] extern crate lazy_static;
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

// private
pub mod ffi;

// private
pub mod core;

mod prelude;
pub use self::prelude::*;

pub mod socket_base;

mod socket_builder;
pub use self::socket_builder::SocketBuilder;

mod socket_listener;
pub use self::socket_listener::SocketListener;

mod stream_socket;
pub use self::stream_socket::StreamSocket;

mod dgram_socket;
pub use self::dgram_socket::DgramSocket;

pub mod generic;

pub mod local;

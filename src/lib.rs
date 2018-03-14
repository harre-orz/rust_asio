// asyncio
//
// The software is released under the MIT license. see LICENSE.txt
// https://github.com/harre-orz/rust_asio/blob/master/LICENSE.txt

#[macro_use]
extern crate bitflags;

#[cfg(feature = "context")]
extern crate context;

extern crate errno;

extern crate kernel32;

#[macro_use]
extern crate lazy_static;

extern crate libc;

// #[cfg(feature = "openssl")]
// extern crate openssl;

#[cfg(feature = "openssl-sys")]
extern crate openssl_sys;

#[cfg(feature = "termios")]
extern crate termios;

#[cfg(feature = "test")]
extern crate test;

extern crate winapi;

extern crate ws2_32;

mod prelude;

mod ffi;

mod core;

mod ops;

pub mod clock;

mod streams;

pub mod socket_base;

mod dgram_socket;

mod stream_socket;

mod socket_listener;

pub mod generic;

pub mod local;

pub mod ip;

mod from_str;

pub mod posix;

#[cfg(unix)]
mod signal_set;

#[cfg(feature = "termios")]
mod serial_port;

//pub mod ssl;

pub use self::prelude::*;

pub use self::core::{AsIoContext, IoContext, IoContextWork};

pub use self::ops::{wrap, ArcHandler, Handler, Strand, StrandHandler, StrandImmutable};

#[cfg(feature = "context")]
pub use self::ops::{spawn, Coroutine, CoroutineHandler};

pub use self::streams::{MatchCond, Stream, StreamBuf};

pub use self::dgram_socket::DgramSocket;

pub use self::stream_socket::StreamSocket;

pub use self::socket_listener::SocketListener;

#[cfg(unix)]
pub use self::signal_set::{Signal, SignalSet, raise};

#[cfg(feature = "termios")]
pub use self::serial_port::{SerialPort, SerialPortOption, BaudRate, Parity, CSize, FlowControl,
                            StopBits};

pub type SteadyTimer = clock::WaitableTimer<clock::SteadyClock>;

pub type SystemTimer = clock::WaitableTimer<clock::SystemClock>;

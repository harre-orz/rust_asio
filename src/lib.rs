// asyncio
//
// The software is released under the MIT license. see LICENSE.txt
// https://github.com/harre-orz/rust_asio/blob/master/LICENSE.txt

#[allow(unused_imports)]
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
pub use self::prelude::*;

mod ffi;

mod core;
pub use self::core::{AsIoContext, IoContext, IoContextWork};

mod handler;
pub use self::handler::{wrap, ArcHandler, Handler, Strand, StrandHandler, StrandImmutable};
pub use self::handler::{spawn, Coroutine, CoroutineHandler};

pub mod clock;
pub type SteadyTimer = clock::WaitableTimer<clock::SteadyClock>;
pub type SystemTimer = clock::WaitableTimer<clock::SystemClock>;

mod streams;
pub use self::streams::{MatchCond, Stream, StreamBuf};

pub mod socket_base;

mod ops;

mod dgram_socket;
pub use self::dgram_socket::DgramSocket;

mod stream_socket;
pub use self::stream_socket::StreamSocket;

mod socket_listener;
pub use self::socket_listener::SocketListener;

pub mod generic;

pub mod local;

pub mod ip;

pub mod posix;

#[cfg(unix)] mod signal_set;
#[cfg(unix)] pub use self::signal_set::{Signal, SignalSet, raise};

#[cfg(feature = "termios")] mod serial_port;
#[cfg(feature = "termios")] pub use self::serial_port::{SerialPort, SerialPortOption, BaudRate, Parity, CSize, FlowControl, StopBits};

//pub mod ssl;

mod from_str;

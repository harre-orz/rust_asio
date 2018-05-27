// asyncio
//
// The software is released under the MIT license. see LICENSE.txt
// https://github.com/harre-orz/rust_asio/blob/master/LICENSE.txt

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate lazy_static;

extern crate libc;

#[cfg(windows)]
extern crate winapi;

#[cfg(feature = "context")]
extern crate context;

#[cfg(feature = "termios")]
extern crate termios;

#[cfg(feature = "test")]
extern crate test;

mod ffi;

// mod timer;

// mod reactor;

// mod core;
// pub use self::core::{AsIoContext, IoContext, IoContextWork, Protocol, Endpoint, Socket, IoControl,
//                      GetSocketOption, SetSocketOption, Cancel};

// mod handler;
// pub use self::handler::{Handler, ArcHandler, wrap};

// mod strand;
// pub use self::strand::*;

// mod accept_ops;

// mod connect_ops;

// mod read_ops;

// mod write_ops;

// pub mod clock;
// pub type SteadyTimer = clock::WaitableTimer<clock::SteadyClock>;
// pub type SystemTimer = clock::WaitableTimer<clock::SystemClock>;

// mod streambuf;
// pub use self::streambuf::*;

// pub mod socket_base;

// mod stream;
// pub use self::stream::*;

// mod dgram_socket;
// pub use self::dgram_socket::*;

// mod stream_socket;
// pub use self::stream_socket::*;

// mod socket_listener;
// pub use self::socket_listener::*;

// pub mod generic;

// pub mod local;

// pub mod ip;

// mod from_str;

// pub mod posix;

// #[cfg(unix)]
// mod signal_set;
// #[cfg(unix)]
// pub use self::signal_set::{Signal, SignalSet, raise};

// #[cfg(feature = "termios")]
// mod serial_port;
// #[cfg(feature = "termios")]
// pub use self::serial_port::{SerialPort, SerialPortOption, BaudRate, Parity, CSize, FlowControl,
//                             StopBits};

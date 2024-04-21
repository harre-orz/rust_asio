extern crate libc;

mod error;
pub use self::error::{Error, ResolverError, Result};

mod socket_base;
pub use self::socket_base::{
    AddressFamily, Endpoint, IntoProtocolType, Protocol, Shutdown, SockaddrType, SocketType,
    SocklenType,
};

mod signal_base;
pub use self::signal_base::Signal;

mod ffi;

mod executor;
pub use self::executor::IoContext;

pub mod dgram;
pub mod listener;
pub mod stream;

pub mod ip;
pub mod local;
pub mod signal_set;

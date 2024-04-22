extern crate libc;

mod socket_base;
pub use self::socket_base::{
    AddressFamily, Endpoint, IntoProtocolType, Protocol, Shutdown, SockaddrType, SocketType,
    SocklenType,
};

mod error;
pub use self::error::{OsError, ResolverError};

mod ffi;

mod executor;
pub use self::executor::IoContext;

mod ops;

pub mod dgram;
pub mod listener;
pub mod stream;

pub mod ip;
pub mod local;
pub mod signal_set;

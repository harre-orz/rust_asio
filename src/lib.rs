pub mod error;

pub mod sockaddr;

pub mod iface;

pub mod socket_base;

pub mod buffer;

pub mod msghdr;

mod primitive;

mod core;
pub use self::core::IoContext;

mod socket;

pub mod stream_socket;

pub mod seqpacket_socket;

pub mod dgram_socket;

pub mod socket_listener;

pub mod ip;

pub mod generic;

pub mod local;

#[cfg(unix)]
pub mod posix;

#[cfg(unix)]
pub mod signal_set;

#[cfg(unix)]
pub mod serial_port;

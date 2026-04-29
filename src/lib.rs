pub mod error;

pub mod sockaddr;

pub mod socket_base;

pub mod buffer;

mod socket;

mod exec;
pub use self::exec::IoContext;

mod ops;

pub mod stream_socket;

pub mod seqpacket_socket;

pub mod dgram_socket;

pub mod socket_listener;

pub mod ip;

pub mod generic;

pub mod local;

#[cfg(unix)]
pub mod posix;

#[cfg(target_os = "linux")]
pub mod signal_set;

#[cfg(unix)]
pub mod serial_port;

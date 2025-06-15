pub mod error;

pub mod socket_base;

mod ffi;

mod exec;
pub use self::exec::IoContext;

mod ops;

pub mod io_stream;

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

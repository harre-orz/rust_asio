extern crate libc;

#[cfg(windows)]
extern crate windows_sys;

pub mod error;
mod executor;
mod sockaddr;
mod socket;
pub mod socket_base;
pub use self::executor::IoContext;
pub mod dgram_socket;
pub mod generic;
pub mod io_stream;
pub mod ip;
pub mod listener;
pub mod local;
mod ops;
pub mod seqpacket_socket;
pub mod stream_socket;

// #[cfg(unix)]
// pub mod posix;
//
// #[cfg(unix)]
// pub mod signal_set;

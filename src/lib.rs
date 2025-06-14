extern crate libc;

#[cfg(windows)]
extern crate windows_sys;

pub mod error;
mod sockaddr;
pub mod socket_base;
mod socket;
mod executor;
pub use self::executor::IoContext;
mod ops;
pub mod stream;
pub mod listener;
pub mod dgram;
pub mod local;
pub mod generic;
pub mod ip;

// #[cfg(unix)]
// pub mod signal_set;

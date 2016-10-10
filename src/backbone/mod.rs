use std::io;
use std::mem;
use std::cmp;
use std::hash;
use std::slice;
use std::boxed::FnBox;
pub use std::os::unix::io::{RawFd, AsRawFd};
pub use libc::{c_void, c_int, c_char, memcmp};
use {IoObject, IoService, SockAddr};
use error_code::{ErrorCode, errno};
use io_service::{IoActor, WaitActor};

mod unix;
pub use self::unix::*;

pub type Handler = Box<FnBox(*const IoService, ErrorCode) + Send + 'static>;

pub trait AsIoActor : IoObject + AsRawFd + 'static {
    fn as_io_actor(&self) -> &IoActor;
}

pub trait AsWaitActor : IoObject + 'static {
    fn as_wait_actor(&self) -> &WaitActor;
}

pub mod ops;

#[cfg(all(not(feature = "asyncio_no_signal_set"), target_os = "linux"))]
pub mod signalfd;

fn eof() -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, "End of File")
}

fn write_zero() -> io::Error {
    io::Error::new(io::ErrorKind::WriteZero, "Write Zero")
}

fn stopped() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Stopped")
}

fn canceled() -> io::Error {
    io::Error::from_raw_os_error(CANCELED)
}

pub fn sockaddr_eq<E: SockAddr>(lhs: &E, rhs: &E) -> bool {
    lhs.size() == rhs.size() && unsafe { memcmp(
        mem::transmute(lhs.as_sockaddr()),
        mem::transmute(rhs.as_sockaddr()),
        lhs.size())
    } == 0
}

pub fn sockaddr_cmp<E: SockAddr>(lhs: &E, rhs: &E) -> cmp::Ordering {
    let cmp = unsafe {
        memcmp(
            mem::transmute(lhs.as_sockaddr()),
            mem::transmute(rhs.as_sockaddr()),
            cmp::min(lhs.size(), rhs.size())
        )
    };
    if cmp == 0 {
        if lhs.size() < rhs.size() {
            cmp::Ordering::Less
        } else if lhs.size() > rhs.size() {
            cmp::Ordering::Greater
        } else {
            cmp::Ordering::Equal
        }
    } else if cmp < 0 {
        cmp::Ordering::Less
    } else {
        cmp::Ordering::Greater
    }
}

pub fn sockaddr_hash<E: SockAddr, H: hash::Hasher>(ep: &E, state: &mut H) {
    let ptr = ep.as_sockaddr() as *const _ as *const u8;
    let buf = unsafe { slice::from_raw_parts(ptr, ep.size()) };
    state.write(buf);
}

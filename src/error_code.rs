use std::io;
use libc::{ECANCELED, c_int};

extern {
    #[cfg_attr(target_os = "linux", link_name = "__errno_location")]
    fn errno_location() -> *mut c_int;
}

pub fn errno() -> i32 {
    unsafe { *errno_location() }
}

pub fn eof() -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, "End of File")
}

pub fn write_zero() -> io::Error {
    io::Error::new(io::ErrorKind::WriteZero, "Write Zero")
}

pub fn stopped() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Stopped")
}

pub fn canceled() -> io::Error {
    io::Error::from_raw_os_error(ECANCELED)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ErrorCode(pub i32);
pub const READY: ErrorCode = ErrorCode(0);
pub const CANCELED: ErrorCode = ErrorCode(ECANCELED);

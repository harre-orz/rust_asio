use std::io;
use std::fmt;
use libc::{ECANCELED};
use errno;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ErrorCode(pub i32);

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", errno::Errno(self.0))
    }
}

pub const READY: ErrorCode = ErrorCode(0);
pub const CANCELED: ErrorCode = ErrorCode(ECANCELED);

pub fn errno() -> i32 {
    errno::errno().0
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

use prelude::{SocketOption, GetSocketOption};
use ffi::{self, AsRawFd, SOL_SOCKET, SO_ERROR, getsockopt};

use std::io;
use std::mem;
use std::fmt;
use errno::{Errno, errno};

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ErrCode(Errno);

impl fmt::Display for ErrCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub const READY: ErrCode = ErrCode(Errno(0));
pub const EINTR: ErrCode = ErrCode(Errno(ffi::EINTR));
pub const EINPROGRESS: ErrCode = ErrCode(Errno(ffi::EINPROGRESS));
pub const ECANCELED: ErrCode = ErrCode(Errno(ffi::ECANCELED));
pub const EAGAIN: ErrCode = ErrCode(Errno(ffi::EAGAIN));
pub const EWOULDBLOCK: ErrCode = ErrCode(Errno(ffi::EWOULDBLOCK));
pub const EAFNOSUPPORT: ErrCode = ErrCode(Errno(ffi::EAFNOSUPPORT));

pub fn last_error() -> ErrCode {
    ErrCode(errno())
}

impl From<ErrCode> for io::Error {
    fn from(ec: ErrCode) -> io::Error {
        debug_assert!((ec.0).0 != 0);
        io::Error::from_raw_os_error(unsafe { mem::transmute( (ec.0).0 ) })
    }
}

pub fn eof() -> io::Error {
    io::Error::new(io::ErrorKind::UnexpectedEof, "End of file")
}

pub fn invalid_argument() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Invalid argument")
}

pub fn write_zero() -> io::Error {
    io::Error::new(io::ErrorKind::WriteZero, "Write zero")
}

pub fn host_not_found() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Host not found")
}

#[derive(Default, Clone)]
struct Error(i32);

impl<P> SocketOption<P> for Error {
    type Data = i32;

    fn level(&self, _: &P) -> i32 { SOL_SOCKET }

    fn name(&self, _: &P) -> i32 { SO_ERROR }
}

impl<P> GetSocketOption<P> for Error {
    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.0
    }
}

pub fn sock_error<T>(t: &T) -> ErrCode
    where T: AsRawFd,
{
    let ec = match getsockopt(t, &()) {
        Ok(Error(ec)) => ec,
        Err(err) => err.raw_os_error().unwrap_or_default(),
    };
    ErrCode(Errno(unsafe { mem::transmute(ec) }))
}

use std::io;
use std::fmt;
use std::mem;
use std::os::unix::io::RawFd;
use libc::{self, SOL_SOCKET, SO_ERROR, c_void, socklen_t, getsockopt};
use errno::{self, Errno};

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ErrCode(errno::Errno);

impl fmt::Display for ErrCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ErrCode> for io::Error {
    fn from(ec: ErrCode) -> io::Error {
        debug_assert!((ec.0).0 != 0);
        io::Error::from_raw_os_error((ec.0).0)
    }
}

pub const READY: ErrCode = ErrCode(Errno(0));
pub const EAGAIN: ErrCode = ErrCode(Errno(libc::EAGAIN));
pub const ECANCELED: ErrCode = ErrCode(Errno(libc::ECANCELED));
pub const EINTR: ErrCode = ErrCode(Errno(libc::EINTR));
pub const EINPROGRESS: ErrCode = ErrCode(Errno(libc::EINPROGRESS));

pub fn errno() -> ErrCode {
    ErrCode(errno::errno())
}

pub fn getsockerr(fd: RawFd) -> ErrCode {
    let mut ec = 0i32;
    let mut len = mem::size_of::<i32>() as socklen_t;
    libc_ign!(getsockopt(fd, SOL_SOCKET, SO_ERROR, &mut ec as *mut _ as *mut c_void, &mut len));
    ErrCode(Errno(ec))
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

pub fn stopped() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Stopped")
}



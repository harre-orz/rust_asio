use ffi::{self, RawFd, SOL_SOCKET, SO_ERROR};

use std::io;
use std::mem;
use std::fmt;
use libc;
use errno::{Errno, errno};


#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct ErrCode(Errno);

impl Default for ErrCode {
    fn default() -> Self {
        ErrCode(Errno(0))
    }
}

// impl fmt::Display for ErrCode {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", self.0)
//     }
// }

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


pub fn sock_error<S>(fd: RawFd) -> ErrCode {
    let mut cmd: libc::c_int = 0;
    let mut cmdlen = mem::size_of::<libc::c_int>() as  libc::socklen_t;
    if unsafe { libc::getsockopt(fd, SOL_SOCKET, SO_ERROR, &mut cmd as *mut _ as *mut _, &mut cmdlen) } != 0 {
        return last_error()
    }
    ErrCode(Errno(cmd))
}

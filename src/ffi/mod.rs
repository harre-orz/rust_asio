use std::io;
use std::result;
use errno::Errno;

#[cfg(unix)] mod posix;
#[cfg(unix)] pub use self::posix::*;

#[cfg(windows)] mod win;
#[cfg(windows)] pub use self::win::*;

mod tss;
pub use self::tss::TssPtr;

mod sa;
pub use self::sa::SockAddr;

mod fdset;
pub use self::fdset::FdSet;

pub fn error(ec: Errno) -> io::Error {
    io::Error::from_raw_os_error(ec.0)
}

pub type Result<T> = result::Result<T, Errno>;

use std::{error, fmt, io, result};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::OsError;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::OsError;

impl fmt::Debug for OsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error {{ code = {} ({}) }}", self.code(), self,)
    }
}

impl error::Error for OsError {}

impl Into<io::Error> for OsError {
    fn into(self) -> io::Error {
        io::Error::from_raw_os_error(self.code())
    }
}

#[cfg(doc)]
pub use crate::primitive::TimeoutError;

use std::{error, fmt, io, result};

#[cfg(unix)]
mod error_unix;
#[cfg(unix)]
pub use self::error_unix::OsError;

#[cfg(windows)]
mod error_windows;
#[cfg(windows)]
pub use self::error_windows::OsError;

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

pub type Result<T> = result::Result<T, OsError>;

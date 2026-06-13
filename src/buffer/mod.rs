use crate::error::OsError;
use std::fmt;
use std::{error, io};

#[derive(Debug)]
#[non_exhaustive]
pub struct TryReserveError;

impl fmt::Display for TryReserveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to reserve memory")
    }
}

impl error::Error for TryReserveError {}

impl From<TryReserveError> for OsError {
    #[cfg(unix)]
    fn from(_: TryReserveError) -> Self {
        Self::NO_MEMORY
    }

    #[cfg(windows)]
    fn from(_: TryReserveError) -> Self {
        Self::NOT_ENOUGH_MEMORY
    }
}

impl Into<io::Error> for TryReserveError {
    fn into(self) -> io::Error {
        OsError::from(self).into()
    }
}

mod sbuf;
pub use self::sbuf::{AsyncIoStream, IoStream, StreamBuf, StreamBufMut};

mod mbuf;
pub use self::mbuf::{MsgBuf, MsgBufMut};

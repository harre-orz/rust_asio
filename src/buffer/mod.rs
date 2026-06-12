use crate::error::OsError;
use std::io;

#[derive(Debug)]
#[non_exhaustive]
pub struct TryReserveError;

impl From<TryReserveError> for OsError {
    #[cfg(unix)]
    fn from(_: TryReserveError) -> Self {
        Self::NO_MEMORY
    }

    #[cfg(windows)]
    fn from(_: OsError) -> Self {
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

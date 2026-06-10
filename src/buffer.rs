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

mod stream;
pub use self::stream::{AsyncIoStream, IoStream, StreamBuf, StreamBufMut};

mod message;
pub use self::message::{MsgBuf, MsgBufMut};

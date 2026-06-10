use super::TryReserveError;
use crate::socket_base::Endpoint;
use std::ops::{Deref, DerefMut};

#[cfg(target_os = "linux")]
mod message_linux;
#[cfg(target_os = "linux")]
pub use self::message_linux::MsgBuf;

#[cfg(target_os = "macos")]
mod message_macos;
#[cfg(target_os = "macos")]
pub use self::message_macos::MsgBuf;

#[cfg(target_os = "windows")]
mod message_windows;
#[cfg(target_os = "windows")]
pub use self::message_windows::MsgBuf;

pub struct MsgBufMut<'a>(&'a mut MsgBuf);

impl<'a> MsgBufMut<'a> {
    pub fn commit<E>(self, len: usize, ep: &E)
    where
        E: Endpoint,
    {
        self.0.commit(len, ep)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.prepare_bytes()
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.0.prepare_bytes()
    }
}

impl<'a> Deref for MsgBufMut<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.prepare_bytes()
    }
}

impl<'a> DerefMut for MsgBufMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.prepare_bytes()
    }
}

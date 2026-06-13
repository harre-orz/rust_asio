use crate::error::{OsError, Result};
use crate::primitive::Timeout;
use std::ptr;
use windows_sys::Win32::Foundation;
use windows_sys::Win32::Networking::WinSock;
use windows_sys::Win32::Storage::FileSystem;
use windows_sys::Win32::System::{IO, Pipes};

pub trait AsRawHandle {
    unsafe fn as_raw_handle(&self) -> Foundation::HANDLE;
}

pub struct Handle(Foundation::HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let _ = Foundation::CloseHandle(self.0);
        }
    }
}

impl Handle {
    pub(crate) unsafe fn from_raw_handle(handle: Foundation::HANDLE) -> Self {
        Self(handle)
    }

    pub(crate) fn write(&self, bytes: &[u8]) -> Result<usize> {
        let mut len = 0;
        unsafe {
            match FileSystem::WriteFile(
                self.0,
                bytes.as_ptr(),
                bytes.len() as u32,
                &mut len,
                ptr::null_mut(),
            ) {
                0 => Err(OsError::last()),
                _ => Ok(len as usize),
            }
        }
    }

    pub(crate) fn read(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut len = 0;
        unsafe {
            match FileSystem::ReadFile(
                self.0,
                bytes.as_mut_ptr(),
                bytes.len() as u32,
                &mut len,
                ptr::null_mut(),
            ) {
                0 => Err(OsError::last()),
                _ => Ok(len as usize),
            }
        }
    }

    pub(crate) fn pipe() -> Result<(Self, Self)> {
        let mut read = ptr::null_mut();
        let mut write = ptr::null_mut();

        unsafe {
            match Pipes::CreatePipe(&mut read, &mut write, ptr::null_mut(), 0) {
                0 => Err(OsError::last()),
                _ => Ok((Self(read), Self(write))),
            }
        }
    }
}

impl AsRawHandle for Handle {
    unsafe fn as_raw_handle(&self) -> Foundation::HANDLE {
        self.0
    }
}

/// Low-level Windows-based socket type.
pub struct Socket(pub(crate) WinSock::SOCKET);

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe {
            WinSock::closesocket(self.0);
        }
    }
}

impl Timeout {
    pub fn poll_in(&self, soc: &Socket) -> Result<()> {
        let mut poll = WinSock::WSAPOLLFD {
            fd: soc.0,
            events: WinSock::POLLIN,
            revents: 0,
        };
        unsafe {
            match WinSock::WSAPoll(&mut poll, 1, self.0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn poll_out(&self, soc: &Socket) -> Result<()> {
        let mut poll = WinSock::WSAPOLLFD {
            fd: soc.0,
            events: WinSock::POLLOUT,
            revents: 0,
        };
        unsafe {
            match WinSock::WSAPoll(&mut poll, 1, self.0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }
}

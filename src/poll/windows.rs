use crate::error::{OsError, Result};
use std::mem::MaybeUninit;
use std::ptr;
use windows_sys::Win32::Foundation;
use windows_sys::Win32::Networking::WinSock;
use windows_sys::Win32::Storage::FileSystem;
use windows_sys::Win32::System::{IO, Pipes};

pub(crate) struct WinSockEx {
    pub ConnectEx: unsafe fn(
        s: WinSock::SOCKET,
        name: *const WinSock::SOCKADDR,
        namelen: i32,
        lpsendbuffer: *const core::ffi::c_void,
        dwsenddatalength: u32,
        lpdwbytessent: *mut u32,
        lpoverlapped: *mut IO::OVERLAPPED,
    ) -> windows_sys::core::BOOL,
    pub WSARecvMsg: unsafe fn(
        s: WinSock::SOCKET,
        lpmsg: *mut WinSock::WSAMSG,
        lpdwnumberofbytesrecvd: *mut u32,
        lpoverlapped: *mut IO::OVERLAPPED,
        lpcompletionroutine: WinSock::LPWSAOVERLAPPED_COMPLETION_ROUTINE,
    ) -> i32,
}

impl Drop for WinSockEx {
    fn drop(&mut self) {
        unsafe {
            let _ = WinSock::WSACleanup();
        }
    }
}

struct SocketGuard(WinSock::SOCKET);

impl Drop for SocketGuard {
    fn drop(&mut self) {
        unsafe {
            WinSock::closesocket(self.0);
        }
    }
}

impl SocketGuard {
    fn ex(&self) -> Result<WinSockEx> {
        unsafe {
            let ConnectEx = {
                let guid = WinSock::WSAID_CONNECTEX;
                let mut lpfn: WinSock::LPFN_CONNECTEX = None;
                let mut bytes = 0;
                match WinSock::WSAIoctl(
                    self.0,
                    WinSock::SIO_GET_EXTENSION_FUNCTION_POINTER,
                    ptr::from_ref(&guid).cast(),
                    size_of_val(&guid) as _,
                    ptr::from_mut(&mut lpfn).cast(),
                    size_of_val(&lpfn) as _,
                    &mut bytes,
                    ptr::null_mut(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => return Err(OsError::last()),
                    _ => lpfn.unwrap(),
                }
            };

            let WSARecvMsg = {
                let guid = WinSock::WSAID_WSARECVMSG;
                let mut lpfn: WinSock::LPFN_WSARECVMSG = None;
                let mut bytes = 0;
                match WinSock::WSAIoctl(
                    self.0,
                    WinSock::SIO_GET_EXTENSION_FUNCTION_POINTER,
                    ptr::from_ref(&guid).cast(),
                    size_of_val(&guid) as _,
                    ptr::from_mut(&mut lpfn).cast(),
                    size_of_val(&lpfn) as _,
                    &mut bytes,
                    ptr::null_mut(),
                    None,
                ) {
                    WinSock::SOCKET_ERROR => return Err(OsError::last()),
                    _ => lpfn.unwrap(),
                }
            };
            Ok(WinSockEx {
                ConnectEx: ConnectEx,
                WSARecvMsg: WSARecvMsg,
            })
        }
    }
}

impl WinSockEx {
    pub(crate) fn new() -> Result<Self> {
        unsafe {
            let mut _data = MaybeUninit::<WinSock::WSADATA>::uninit();
            match WinSock::WSAStartup(0x0202, _data.as_mut_ptr()) {
                0 => {
                    match WinSock::WSASocketW(
                        WinSock::AF_INET as i32,
                        WinSock::SOCK_STREAM,
                        WinSock::IPPROTO_IP,
                        ptr::null_mut(),
                        0,
                        0,
                    ) {
                        WinSock::INVALID_SOCKET => Err(OsError::last()),
                        soc => Ok(SocketGuard(soc).ex()?),
                    }
                }
                err => Err(OsError::from_raw(err)),
            }
        }
    }
}

pub(crate) trait AsRawHandle {
    unsafe fn as_raw_handle(&self) -> Foundation::HANDLE;
}

pub(crate) struct Handle(Foundation::HANDLE);

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

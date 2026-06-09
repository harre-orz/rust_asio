use super::Timeout;
use crate::IoContext;
use crate::buffer::MsgBuf;
use crate::error::{OsError, Result};
use crate::sockaddr::{SockAddr, SockLen};
use crate::socket_base::{Endpoint, EndpointRef, GetSockOpt, Protocol, SetSockOpt, Shutdown};
use std::mem::MaybeUninit;
use std::ptr;
use windows_sys::Win32::Foundation;
use windows_sys::Win32::Networking::WinSock;
use windows_sys::Win32::Storage::FileSystem;
use windows_sys::Win32::System::{IO, Pipes};

pub(crate) struct Handle(Foundation::HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let _ = Foundation::CloseHandle(self.0);
        }
    }
}

pub(crate) trait AsHandle {
    unsafe fn as_raw_handle(&self) -> Foundation::HANDLE;
}

impl Handle {
    pub(crate) unsafe fn new_unchecked(handle: Foundation::HANDLE) -> Self {
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

impl AsHandle for Handle {
    unsafe fn as_raw_handle(&self) -> Foundation::HANDLE {
        self.0
    }
}

const SOCKET_ERROR_: WinSock::SOCKET = WinSock::SOCKET_ERROR as WinSock::SOCKET;

fn set_nonblock(soc: &Socket) -> Result<()> {
    let mut val = 0;
    unsafe {
        match WinSock::ioctlsocket(soc.0, WinSock::FIONBIO, &mut val) {
            WinSock::SOCKET_ERROR => Err(OsError::last()),
            _ => Ok(()),
        }
    }
}

/// Low-level Windows-based socket type.
pub struct Socket(WinSock::SOCKET);

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe {
            WinSock::closesocket(self.0);
        }
    }
}

impl Socket {
    pub fn new<P>(pro: P) -> Result<Self>
    where
        P: Protocol,
    {
        unsafe {
            match WinSock::WSASocketW(
                pro.family_type().into(),
                pro.socket_type().into(),
                pro.protocol_type().into(),
                ptr::null_mut(),
                0,
                WinSock::WSA_FLAG_OVERLAPPED,
            ) {
                SOCKET_ERROR_ => Err(OsError::last()),
                soc => {
                    let soc = Socket(soc);
                    set_nonblock(&soc)?;
                    Ok(soc)
                }
            }
        }
    }

    pub(crate) unsafe fn as_raw_socket(&self) -> WinSock::SOCKET {
        self.0
    }

    pub fn close(self) -> Result<()> {
        unsafe {
            match WinSock::closesocket(self.0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn bind<E>(&self, ep: &EndpointRef<E>) -> Result<()>
    where
        E: Endpoint,
    {
        let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
        unsafe {
            match WinSock::bind(self.0, sa, ep.sockaddr_len()) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn listen(&self, backlog: i32) -> Result<()> {
        unsafe {
            match WinSock::listen(self.0, backlog) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn connect<E>(&self, ep: &EndpointRef<E>) -> Result<()>
    where
        E: Endpoint,
    {
        let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
        unsafe {
            match WinSock::connect(self.0, sa, ep.sockaddr_len()) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn accept<E>(&self) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match WinSock::accept(self.0, sa.as_mut_ptr().cast(), &mut sa_len) {
                SOCKET_ERROR_ => Err(OsError::last()),
                soc => {
                    let soc = Socket(soc);
                    let ep = E::from_sockaddr(E::SockAddr::init(sa, sa_len));
                    Ok((soc, ep))
                }
            }
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        self.receive(buf)
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match WinSock::recv(self.0, buf.as_mut_ptr().cast(), buf.len() as i32, 0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn receive_msg(&self, mbuf: &mut MsgBuf, ctx: &IoContext) -> Result<usize> {
        let mut len = MaybeUninit::<u32>::uninit();
        unsafe {
            match (ctx.winsock().WSARecvMsg)(
                self.0,
                mbuf.as_ptr(),
                len.as_mut_ptr(),
                ptr::null_mut(),
                None,
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => {
                    let len = len.assume_init();
                    if len == 0 {
                        Err(OsError::CONNECTION_ABORTED)
                    } else {
                        Ok(len as usize)
                    }
                }
            }
        }
    }

    pub fn receive_from<E>(&self, buf: &mut [u8]) -> Result<(usize, E)>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match WinSock::recvfrom(
                self.0,
                buf.as_mut_ptr().cast(),
                buf.len() as i32,
                0,
                sa.as_mut_ptr().cast(),
                &mut sa_len,
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => {
                    let sa = E::SockAddr::init(sa, sa_len);
                    Ok((len as usize, E::from_sockaddr(sa)))
                }
            }
        }
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        unsafe {
            match WinSock::send(self.0, buf.as_ptr().cast(), buf.len() as i32, 0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn send_to<E>(&self, buf: &[u8], ep: &EndpointRef<E>) -> Result<usize>
    where
        E: Endpoint,
    {
        unsafe {
            let sa = ptr::from_ref(ep.sockaddr_ref()).cast();
            match WinSock::sendto(
                self.0,
                buf.as_ptr().cast(),
                buf.len() as i32,
                0,
                sa,
                ep.sockaddr_len(),
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        unsafe {
            match WinSock::WSASendMsg(
                self.0,
                mbuf.as_ptr(),
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                None,
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        self.send(buf)
    }

    pub fn getsockname<E>(&self) -> Result<E>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match WinSock::getsockname(self.0, sa.as_mut_ptr().cast(), &mut sa_len) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => {
                    let ep = E::SockAddr::init(sa, sa_len);
                    Ok(E::from_sockaddr(ep))
                }
            }
        }
    }

    pub fn getpeername<E>(&self) -> Result<E>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match WinSock::getpeername(self.0, sa.as_mut_ptr().cast(), &mut sa_len) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => {
                    let ep = E::SockAddr::init(sa, sa_len);
                    Ok(E::from_sockaddr(ep))
                }
            }
        }
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        unsafe {
            match WinSock::shutdown(self.0, how as WinSock::WINSOCK_SHUTDOWN_HOW) {
                WinSock::SOCKET_ERROR => Err(unsafe { OsError::last() }),
                _ => Ok(()),
            }
        }
    }

    pub fn getsockopt<P, S>(&self, pro: P) -> Result<S>
    where
        P: Protocol,
        S: GetSockOpt<P>,
    {
        let (key, init) = S::init(pro);
        let mut data = MaybeUninit::<S>::uninit();
        let mut data_len = size_of::<S>() as SockLen;
        unsafe {
            match WinSock::getsockopt(
                self.0,
                key.level,
                key.name,
                data.as_mut_ptr().cast(),
                &mut data_len,
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(init(data, data_len)),
            }
        }
    }

    pub fn setsockopt<P>(&self, pro: P, opt: &dyn SetSockOpt<P>) -> Result<()>
    where
        P: Protocol,
    {
        let (key, data) = opt.data(pro);
        unsafe {
            match WinSock::setsockopt(
                self.0,
                key.level,
                key.name,
                data.as_ptr().cast(),
                data.len() as SockLen,
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }
    pub fn poll_in(&self, timeout: Timeout) -> Result<()> {
        let mut poll = WinSock::WSAPOLLFD {
            fd: self.0,
            events: WinSock::POLLIN,
            revents: 0,
        };
        unsafe {
            match WinSock::WSAPoll(&mut poll, 1, timeout.0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn poll_out(&self, timeout: Timeout) -> Result<()> {
        let mut poll = WinSock::WSAPOLLFD {
            fd: self.0,
            events: WinSock::POLLOUT,
            revents: 0,
        };
        unsafe {
            match WinSock::WSAPoll(&mut poll, 1, timeout.0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }
}

impl AsHandle for Socket {
    unsafe fn as_raw_handle(&self) -> Foundation::HANDLE {
        self.0 as Foundation::HANDLE
    }
}

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

fn startup() -> Result<()> {
    let mut _data = MaybeUninit::<WinSock::WSADATA>::uninit();
    unsafe {
        match WinSock::WSAStartup(0x0202, _data.as_mut_ptr()) {
            0 => Ok(()),
            _ => Err(OsError::last()),
        }
    }
}

impl WinSockEx {
    pub(crate) fn new() -> Result<Self> {
        startup()?;
        unsafe {
            match WinSock::WSASocketW(
                WinSock::AF_INET as i32,
                WinSock::SOCK_STREAM,
                WinSock::IPPROTO_IP,
                ptr::null_mut(),
                0,
                0,
            ) {
                SOCKET_ERROR_ => Err(OsError::last()),
                soc => {
                    let soc = Socket(soc);

                    let ConnectEx = {
                        let guid = WinSock::WSAID_CONNECTEX;
                        let mut lpfn: WinSock::LPFN_CONNECTEX = None;
                        let mut bytes = 0;
                        match WinSock::WSAIoctl(
                            soc.as_raw_socket(),
                            WinSock::SIO_GET_EXTENSION_FUNCTION_POINTER,
                            ptr::from_ref(&guid).cast(),
                            size_of_val(&guid) as _,
                            ptr::from_mut(&mut lpfn).cast(),
                            size_of_val(&lpfn) as _,
                            &mut bytes,
                            ptr::null_mut(),
                            None,
                        ) {
                            SOCKET_ERROR_ => return Err(OsError::last()),
                            _ => lpfn.unwrap(),
                        }
                    };

                    let WSARecvMsg = {
                        let guid = WinSock::WSAID_WSARECVMSG;
                        let mut lpfn: WinSock::LPFN_WSARECVMSG = None;
                        let mut bytes = 0;
                        match WinSock::WSAIoctl(
                            soc.as_raw_socket(),
                            WinSock::SIO_GET_EXTENSION_FUNCTION_POINTER,
                            ptr::from_ref(&guid).cast(),
                            size_of_val(&guid) as _,
                            ptr::from_mut(&mut lpfn).cast(),
                            size_of_val(&lpfn) as _,
                            &mut bytes,
                            ptr::null_mut(),
                            None,
                        ) {
                            SOCKET_ERROR_ => return Err(OsError::last()),
                            _ => lpfn.unwrap(),
                        }
                    };
                    Ok(Self {
                        ConnectEx: ConnectEx,
                        WSARecvMsg: WSARecvMsg,
                    })
                }
            }
        }
    }
}

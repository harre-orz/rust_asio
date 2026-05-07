use crate::buffer::MsgBuf;
use crate::error::OsError;
use crate::sockaddr::{SockAddr, SockLen};
use crate::socket_base::{Endpoint, EndpointRef, GetSockOpt, Protocol, SetSockOpt, Shutdown};
use std::mem::MaybeUninit;
use std::ptr;
use windows_sys::Win32::Networking::WinSock;
use std::result;

const SOCKET_ERROR: WinSock::SOCKET = WinSock::SOCKET_ERROR as WinSock::SOCKET;

pub struct SocketType(i32);

impl SocketType {
    pub const SOCK_STREAM: Self = Self(WinSock::SOCK_STREAM);
    pub const SOCK_DGRAM: Self = Self(WinSock::SOCK_DGRAM);
    pub const SOCK_RAW: Self = Self(WinSock::SOCK_RAW);
    pub const SOCK_SEQPACKET: Self = Self(WinSock::SOCK_SEQPACKET);
}

impl Into<i32> for SocketType {
    fn into(self) -> i32 {}
}

type Result<T> = result::Result<T, OsError>;

fn set_nonblock(soc: &Socket) -> Result<()> {
    let mut val = 0;
    unsafe {
        match WinSock::ioctlsocket(soc.0, WinSock::FIONBIO, &mut val) {
            WinSock::SOCKET_ERROR => Err(unsafe { OsError::last() }),
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
            match WinSock::socket(pro.family_type().into(), pro.socket_type().into(), pro.protocol_type()) {
                SOCKET_ERROR => Err(OsError::last()),
                soc => {
                    let soc = Socket(soc);
                    set_nonblock(&soc)?;
                    Ok(soc)
                }
            }
        }
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
        unsafe {
            match WinSock::bind(self.0, ep.sockaddr_ref(), ep.sockaddr_len()) {
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
        unsafe {
            match WinSock::connect(self.0, ep.sockaddr_ref().as_raw(), ep.sockaddr_len()) {
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
                SOCKET_ERROR => Err(OsError::last()),
                soc => {
                    let ep = E::SockAddr::init(sa, sa_len);
                    Ok((Socket(soc), ep))
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

    pub fn receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        unimplemented!("WSARecvMsg() is undefined for 'windows-sys'")
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
                    let sa = unsafe { E::SockAddr::init(sa, sa_len) };
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
            match WinSock::sendto(
                self.0,
                buf.as_ptr().cast(),
                buf.len() as i32,
                0,
                ep.sockaddr_ref().as_raw(),
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
            match WinSock::WSASendMsg(self.0, mbuf.as_msghdr_ptr(), 0, 0, 0, 0) {
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
                WinSock::SOCKET_ERROR => Err(unsafe { OsError::last() }),
                _ => {
                    let ep = unsafe { E::SockAddr::init(sa, sa_len) };
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
                WinSock::SOCKET_ERROR => Err(unsafe { OsError::last() }),
                _ => {
                    let ep = unsafe { E::SockAddr::init(sa, sa_len) };
                    Ok(E::from_sockaddr(ep))
                }
            }
        }
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        unsafe {
            match WinSock::shutdown(self.0, how.into()) {
                WinSock::SOCKET_ERROR => Err(unsafe { OsError::last() }),
                _ => Ok(()),
            }
        }
    }

    pub fn getsockopt<S>(&self, sockopt: &S) -> Result<()>
    where
        S: GetSockOpt,
    {
        let (level, name) = S::KEY;
        unsafe {
            let opt_ptr = ptr::from_ref(sockopt) as windows_sys::core::PCSTR;
            match WinSock::setsockopt(self.0, level, name, opt_ptr, sockopt.len() as i32) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn setsockopt<S>(&self, sockopt: &S) -> Result<()>
    where
        S: SetSockOpt,
    {
        let (level, name) = S::KEY;
        unsafe {
            let opt_ptr = ptr::from_ref(sockopt) as windows_sys::core::PCSTR;
            match WinSock::setsockopt(self.0, level, name, opt_ptr, sockopt.len() as i32) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }
}

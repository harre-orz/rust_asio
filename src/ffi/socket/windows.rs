use crate::error::OsError;
use crate::socket_base::{Endpoint, Protocol, Shutdown, SockAddr};
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr;
use std::time::Instant;
use windows_sys::Win32::Networking::WinSock;

type Result<T> = std::result::Result<T, OsError>;

const SOCKET_ERROR: WinSock::SOCKET = -1isize as WinSock::SOCKET;

fn set_nonblock(soc: &Socket) -> Result<()> {
    let mut val = 0;
    unsafe {
        match WinSock::ioctlsocket(soc.0, WinSock::FIONBIO, &mut val) {
            WinSock::SOCKET_ERROR => Err(unsafe { OsError::last() }),
            _ => Ok(()),
        }
    }
}

trait SocketOption: Sized {
    const MAX_SIZE: WinSock::socklen_t = size_of::<Self>() as WinSock::socklen_t;

    fn as_ptr(&self) -> *const libc::c_void {
        ptr::from_ref(self).cast()
    }

    fn len(&self) -> WinSock::socklen_t {
        size_of_val(self) as WinSock::socklen_t
    }

    unsafe fn init(data: MaybeUninit<Self>, len: WinSock::socklen_t) -> Self {
        assert_eq!(len, Self::MAX_SIZE);
        data.assume_init()
    }
}

impl SocketOption for i32 {}

fn setsockopt<T>(soc: &Socket, level: i32, name: i32, data: T) -> Result<()>
where
    T: SocketOption,
{
    unsafe {
        match WinSock::setsockopt(soc.0, level, name, data.as_ptr().cast(), data.len()) {
            -1 => Err(unsafe { OsError::last() }),
            _ => Ok(()),
        }
    }
}

fn getsockopt<T>(soc: &Socket, level: i32, name: i32) -> Result<T>
where
    T: SocketOption,
{
    let mut data = MaybeUninit::<T>::uninit();
    let mut len = T::MAX_SIZE;
    unsafe {
        match WinSock::getsockopt(soc.0, level, name, data.as_mut_ptr().cast(), &mut len) {
            -1 => Err(unsafe { OsError::last() }),
            _ => Ok(unsafe { T::init(data, len) }),
        }
    }
}

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
            match libc::socket(
                pro.family_type().into(),
                pro.socket_type().into(),
                pro.protocol_type().into(),
            ) {
                SOCKET_ERROR => Err(OsError::last()),
                soc => Ok(Socket(soc)),
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

    pub fn bind<E>(&self, ep: &E) -> Result<()>
    where
        E: Endpoint,
    {
        let sa = ep.sockaddr();
        unsafe {
            match WinSock::bind(self.0, sa.as_ptr(), sa.len()) {
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

    pub fn nb_connect<E>(&self, ep: &E) -> Result<()>
    where
        E: Endpoint,
    {
        let sa = ep.sockaddr();
        unsafe {
            match WinSock::connect(self.0, sa.as_ptr(), sa.len()) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn nb_accept<E>(&self) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut salen = E::SockAddr::MAX_SIZE;
        unsafe {
            match WinSock::accept(self.0, sa.as_mut_ptr().cast(), &mut salen) {
                SOCKET_ERROR => Err(OsError::last()),
                soc => {
                    let soc = Socket(soc);
                    let sa = E::SockAddr::init(sa, salen);
                    Ok((soc, E::new(sa)))
                }
            }
        }
    }

    pub fn nb_read(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match WinSock::recv(self.0, buf.as_mut_ptr().cast(), buf.len() as i32, 0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match WinSock::recv(self.0, buf.as_mut_ptr().cast(), buf.len() as i32, 0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn nb_receive_from<E>(&self, buf: &mut [u8]) -> Result<(usize, E)>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut salen = E::SockAddr::MAX_SIZE;
        unsafe {
            match WinSock::recvfrom(
                self.0,
                buf.as_mut_ptr().cast(),
                buf.len() as i32,
                0,
                sa.as_mut_ptr().cast(),
                &mut salen,
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => {
                    let sa = unsafe { E::SockAddr::init(sa, salen) };
                    Ok((len as usize, E::new(sa)))
                }
            }
        }
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        unsafe {
            match WinSock::send(self.0, buf.as_ptr().cast(), buf.len() as i32, 0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn nb_send_to<E>(&self, buf: &[u8], ep: &E) -> Result<usize>
    where
        E: Endpoint,
    {
        let sa = ep.sockaddr();
        unsafe {
            match WinSock::sendto(
                self.0,
                buf.as_ptr().cast(),
                buf.len() as i32,
                0,
                sa.as_ptr(),
                sa.len(),
            ) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn nb_write(&self, buf: &[u8]) -> Result<usize> {
        unsafe {
            match WinSock::send(self.0, buf.as_ptr().cast(), buf.len() as i32, 0) {
                WinSock::SOCKET_ERROR => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn wait_for_readable(&self, time: Option<Instant>) -> Result<()> {
        todo!()
    }

    pub fn wait_for_writable(&self, time: Option<Instant>) -> Result<()> {
        todo!()
    }

    pub fn getsockname<E>(&self) -> Result<E>
    where
        E: Endpoint,
    {
        todo!()
    }

    pub fn getpeername<E>(&self) -> Result<E>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut salen = E::SockAddr::MAX_SIZE;
        unsafe {
            match WinSock::getpeername(self.0, sa.as_mut_ptr().cast(), &mut salen) {
                WinSock::SOCKET_ERROR => Err(unsafe { OsError::last() }),
                _ => {
                    let sa = unsafe { E::SockAddr::init(sa, salen) };
                    Ok(E::new(sa))
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

    pub fn reuse_addr(&self, on: bool) -> Result<()> {
        let on = if on { 1i32 } else { 0i32 };
        setsockopt(self, WinSock::SOL_SOCKET, WinSock::SO_REUSEADDR, on)?;
        Ok(())
    }
}

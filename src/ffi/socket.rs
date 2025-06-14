use crate::error::OsError;
use crate::ffi::SockAddr;
use crate::socket_base::{Endpoint, Protocol, Shutdown};
use std::mem::{self, MaybeUninit};
use std::os::fd::{AsRawFd, RawFd};
use std::ptr;
use std::result;
use std::time::Instant;

type Result<T> = result::Result<T, OsError>;



pub struct Socket(RawFd);

impl Drop for Socket {
    fn drop(&mut self) {
        let _ = close(self);
    }
}

impl Socket {
    pub fn close(mut self) -> Result<()> {
        let res = close(&mut self);
        mem::forget(self);
        res
    }
}

impl AsRawFd for Socket {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

fn close(soc: &Socket) -> Result<()> {
    match unsafe { libc::close(soc.0) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(()),
        _ => unreachable!(),
    }
}

#[cfg(target_os = "linux")]
pub fn socket<P>(pro: P) -> Result<Socket>
where
    P: Protocol,
{
    let socktype: i32 = pro.socket_type().into();
    match unsafe {
        libc::socket(
            pro.family_type().into(),
            socktype | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            pro.protocol_type().into(),
        )
    } {
        -1 => Err(unsafe { OsError::last() }),
        soc => Ok(Socket(soc)),
    }
}

#[cfg(target_os = "macos")]
pub fn socket<P>(pro: P) -> Result<Socket>
where
    P: Protocol,
{
    let socktype: i32 = pro.socket_type().into();
    match unsafe {
        libc::socket(
            pro.family_type().into(),
            socktype,
            pro.protocol_type().into(),
        )
    } {
        -1 => Err(unsafe { OsError::last() }),
        soc => socket_init(Socket(soc)),
    }
}

#[cfg(target_os = "linux")]
pub fn socketpair<P>(pro: P) -> Result<(Socket, Socket)>
where
    P: Protocol,
{
    let mut sv = MaybeUninit::<[RawFd; 2]>::uninit();
    let socktype: i32 = pro.socket_type().into();
    match unsafe {
        libc::socketpair(
            pro.family_type().into(),
            socktype | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            pro.protocol_type().into(),
            sv.as_mut_ptr().cast(),
        )
    } {
        -1 => Err(unsafe { OsError::last() }),
        0 => {
            let sv = unsafe { sv.assume_init() };
            let s1 = Socket(sv[0]);
            let s2 = Socket(sv[1]);
            Ok((s1, s2))
        }
        _ => unreachable!(),
    }
}

#[cfg(target_os = "macos")]
pub fn socketpair<P>(pro: P) -> Result<(Socket, Socket)>
where
    P: Protocol,
{
    let mut sv = MaybeUninit::<[RawFd; 2]>::uninit();
    let socktype: i32 = pro.socket_type().into();
    match unsafe {
        libc::socketpair(
            pro.family_type().into(),
            socktype,
            pro.protocol_type().into(),
            sv.as_mut_ptr().cast(),
        )
    } {
        -1 => Err(unsafe { OsError::last() }),
        0 => {
            let sv = unsafe { sv.assume_init() };
            let s1 = socket_init(Socket(sv[0]))?;
            let s2 = socket_init(Socket(sv[1]))?;
            Ok((s1, s2))
        }
        _ => unreachable!(),
    }
}

pub fn bind<E>(soc: &Socket, ep: &E) -> Result<()>
where
    E: Endpoint,
{
    let sa = ep.sockaddr();
    match unsafe { libc::bind(soc.0, sa.as_ptr(), sa.len()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(()),
        _ => unreachable!(),
    }
}

pub fn listen(soc: &Socket, backlog: i32) -> Result<()> {
    match unsafe { libc::listen(soc.0, backlog) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(()),
        _ => unreachable!(),
    }
}

pub fn connect<E>(soc: &Socket, ep: &E) -> Result<()>
where
    E: Endpoint,
{
    let sa = ep.sockaddr();
    match unsafe { libc::connect(soc.0, sa.as_ptr(), sa.len()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(()),
        _ => unreachable!(),
    }
}

#[cfg(target_os = "linux")]
pub fn accept<E>(soc: &Socket) -> Result<(Socket, E)>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E::SockAddr>::uninit();
    let mut salen = E::SockAddr::MAX_SIZE;
    match unsafe {
        libc::accept4(
            soc.0,
            sa.as_mut_ptr().cast(),
            &mut salen,
            libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
        )
    } {
        -1 => Err(unsafe { OsError::last() }),
        soc => {
            let soc = Socket(soc);
            let sa = unsafe { E::SockAddr::init(sa, salen) };
            Ok((soc, E::new(sa)))
        }
    }
}

#[cfg(target_os = "macos")]
pub fn accept<E>(soc: &Socket) -> Result<(Socket, E)>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E::SockAddr>::uninit();
    let mut salen = E::SockAddr::MAX_SIZE;
    match unsafe { libc::accept(soc.0, sa.as_mut_ptr().cast(), &mut salen) } {
        -1 => Err(unsafe { OsError::last() }),
        soc => {
            let soc = socket_init(Socket(soc))?;
            let sa = unsafe { E::SockAddr::init(sa, salen) };
            Ok((soc, E::new(sa)))
        }
    }
}

pub fn read(soc: &Socket, buf: &mut [u8]) -> Result<usize> {
    match unsafe { libc::read(soc.0, buf.as_mut_ptr().cast(), buf.len()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Err(OsError::CONNECTION_ABORTED),
        len => Ok(len as usize),
    }
}

pub fn receive(soc: &Socket, buf: &mut [u8]) -> Result<usize> {
    match unsafe { libc::recv(soc.0, buf.as_mut_ptr().cast(), buf.len(), 0) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Err(OsError::CONNECTION_ABORTED),
        len => Ok(len as usize),
    }
}

pub fn receive_from<E>(soc: &Socket, buf: &mut [u8]) -> Result<(usize, E)>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E::SockAddr>::uninit();
    let mut salen = E::SockAddr::MAX_SIZE;
    match unsafe {
        libc::recvfrom(
            soc.0,
            buf.as_mut_ptr().cast(),
            buf.len(),
            0,
            sa.as_mut_ptr().cast(),
            &mut salen,
        )
    } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Err(OsError::CONNECTION_ABORTED),
        len => {
            let sa = unsafe { E::SockAddr::init(sa, salen) };
            Ok((len as usize, E::new(sa)))
        }
    }
}

pub fn send(soc: &Socket, buf: &[u8]) -> Result<usize> {
    match unsafe { libc::send(soc.0, buf.as_ptr().cast(), buf.len(), 0) } {
        -1 => Err(unsafe { OsError::last() }),
        0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
        len => Ok(len as usize),
    }
}

pub fn send_to<E>(soc: &Socket, buf: &[u8], ep: &E) -> Result<usize>
where
    E: Endpoint,
{
    let sa = ep.sockaddr();
    match unsafe {
        libc::sendto(
            soc.0,
            buf.as_ptr().cast(),
            buf.len(),
            0,
            sa.as_ptr(),
            sa.len(),
        )
    } {
        -1 => Err(unsafe { OsError::last() }),
        0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
        len => Ok(len as usize),
    }
}

pub fn write(soc: &Socket, buf: &[u8]) -> Result<usize> {
    match unsafe { libc::write(soc.0, buf.as_ptr().cast(), buf.len()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
        len => Ok(len as usize),
    }
}

pub fn wait_for_readable(soc: &Socket, time: Option<Instant>) -> Result<()> {
    let mut poll = libc::pollfd {
        fd: soc.0,
        events: libc::POLLIN,
        revents: 0,
    };
    match unsafe { libc::poll(&mut poll, 1, into_poll(time)) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Err(OsError::OPERATION_CANCELED),
        _ => Ok(()),
    }
}

pub fn wait_for_writable(soc: &Socket, time: Option<Instant>) -> Result<()> {
    let mut poll = libc::pollfd {
        fd: soc.0,
        events: libc::POLLOUT,
        revents: 0,
    };
    match unsafe { libc::poll(&mut poll, 1, into_poll(time)) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Err(OsError::OPERATION_CANCELED),
        _ => Ok(()),
    }
}

pub fn getsockname<E>(soc: &Socket) -> Result<E>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E::SockAddr>::uninit();
    let mut salen = E::SockAddr::MAX_SIZE;
    match unsafe { libc::getsockname(soc.0, sa.as_mut_ptr().cast(), &mut salen) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => {
            let sa = unsafe { E::SockAddr::init(sa, salen) };
            Ok(E::new(sa))
        }
        _ => unreachable!(),
    }
}

pub fn getpeername<E>(soc: &Socket) -> Result<E>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E::SockAddr>::uninit();
    let mut salen = E::SockAddr::MAX_SIZE;
    match unsafe { libc::getpeername(soc.0, sa.as_mut_ptr().cast(), &mut salen) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => {
            let sa = unsafe { E::SockAddr::init(sa, salen) };
            Ok(E::new(sa))
        }
        _ => unreachable!(),
    }
}

pub fn shutdown(soc: &Socket, how: Shutdown) -> Result<()> {
    match unsafe { libc::shutdown(soc.0, how.into()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(()),
        _ => unreachable!(),
    }
}

trait SocketOption: Sized {
    const MAX_SIZE: libc::socklen_t = size_of::<Self>() as libc::socklen_t;

    fn as_ptr(&self) -> *const libc::c_void {
        ptr::from_ref(self).cast()
    }

    fn len(&self) -> libc::socklen_t {
        size_of_val(self) as libc::socklen_t
    }

    unsafe fn init(data: MaybeUninit<Self>, len: libc::socklen_t) -> Self {
        assert_eq!(len, Self::MAX_SIZE);
        data.assume_init()
    }
}

impl SocketOption for i32 {}

fn setsockopt<T>(soc: &Socket, level: i32, name: i32, data: T) -> Result<()>
where
    T: SocketOption,
{
    match unsafe { libc::setsockopt(soc.0, level, name, data.as_ptr(), data.len()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(()),
        _ => unreachable!(),
    }
}

fn getsockopt<T>(soc: &Socket, level: i32, name: i32) -> Result<T>
where
    T: SocketOption,
{
    let mut data = MaybeUninit::<T>::uninit();
    let mut len = T::MAX_SIZE;
    match unsafe { libc::getsockopt(soc.0, level, name, data.as_mut_ptr().cast(), &mut len) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(unsafe { T::init(data, len) }),
        _ => unreachable!(),
    }
}

pub fn reuse_addr(soc: &Socket, on: bool) -> Result<()> {
    let on = if on { 1i32 } else { 0i32 };
    setsockopt(soc, libc::SOL_SOCKET, libc::SO_REUSEADDR, on)?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn signalfd(mask: &libc::sigset_t) -> Result<Socket> {
    match unsafe { libc::signalfd(-1, mask, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC) } {
        -1 => Err(unsafe { OsError::last() }),
        sfd => Ok(Socket(sfd)),
    }
}

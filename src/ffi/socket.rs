use super::{Signal, Timeout};
use crate::error::OsError;
use crate::socket_base::{Endpoint, Protocol, Shutdown, SocklenType};
use std::mem::{self, MaybeUninit};
use std::os::fd::{AsRawFd, RawFd};
use std::result;

type Result<T> = result::Result<T, OsError>;

pub struct ConnectedSocket(RawFd);

impl Drop for ConnectedSocket {
    fn drop(&mut self) {
        let _ = close(self);
    }
}

impl ConnectedSocket {
    pub fn close(mut self) -> Result<()> {
        let res = close(&mut self);
        mem::forget(self);
        res
    }
}

impl AsRawFd for ConnectedSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

pub trait IntoSocket {
    type Socket;

    fn into_socket(&self, soc: ConnectedSocket) -> Self::Socket;
}

fn close(soc: &ConnectedSocket) -> Result<()> {
    match unsafe { libc::close(soc.0) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(()),
        _ => unreachable!(),
    }
}

pub fn socket<P>(pro: P) -> Result<ConnectedSocket>
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
        soc => Ok(ConnectedSocket(soc)),
    }
}

pub fn socketpair<P>(pro: P) -> Result<(ConnectedSocket, ConnectedSocket)>
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
            let s1 = ConnectedSocket(sv[0]);
            let s2 = ConnectedSocket(sv[1]);
            Ok((s1, s2))
        }
        _ => unreachable!(),
    }
}

pub fn bind<E>(soc: &ConnectedSocket, ep: &E) -> Result<()>
where
    E: Endpoint,
{
    match unsafe { libc::bind(soc.0, ep.as_ptr(), ep.len()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(()),
        _ => unreachable!(),
    }
}

pub fn listen(soc: &ConnectedSocket, backlog: i32) -> Result<()> {
    match unsafe { libc::listen(soc.0, backlog) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(()),
        _ => unreachable!(),
    }
}

pub fn connect<E>(soc: &ConnectedSocket, ep: &E) -> Result<()>
where
    E: Endpoint,
{
    match unsafe { libc::connect(soc.0, ep.as_ptr(), ep.len()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(()),
        _ => unreachable!(),
    }
}

pub fn accept<E>(soc: &ConnectedSocket) -> Result<(ConnectedSocket, E)>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
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
            let ep = unsafe { E::init(sa, salen) };
            Ok((ConnectedSocket(soc), ep))
        }
    }
}

pub fn read(soc: &ConnectedSocket, buf: &mut [u8]) -> Result<usize> {
    match unsafe { libc::read(soc.0, buf.as_mut_ptr().cast(), buf.len()) } {
        0 => Err(OsError::CONNECTION_ABORTED),
        -1 => Err(unsafe { OsError::last() }),
        len => Ok(len as usize),
    }
}

pub fn receive(soc: &ConnectedSocket, buf: &mut [u8]) -> Result<usize> {
    match unsafe { libc::recv(soc.0, buf.as_mut_ptr().cast(), buf.len(), 0) } {
        0 => Err(OsError::CONNECTION_ABORTED),
        -1 => Err(unsafe { OsError::last() }),
        len => Ok(len as usize),
    }
}

pub fn receive_from<E>(soc: &ConnectedSocket, buf: &mut [u8]) -> Result<(usize, E)>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
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
        0 => Err(OsError::CONNECTION_ABORTED),
        -1 => Err(unsafe { OsError::last() }),
        len => {
            let ep = unsafe { E::init(sa, salen) };
            Ok((len as usize, ep))
        }
    }
}

pub fn send(soc: &ConnectedSocket, buf: &[u8]) -> Result<usize> {
    match unsafe { libc::send(soc.0, buf.as_ptr().cast(), buf.len(), 0) } {
        0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
        -1 => Err(unsafe { OsError::last() }),
        len => Ok(len as usize),
    }
}

pub fn send_to<E>(soc: &ConnectedSocket, buf: &[u8], ep: &E) -> Result<usize>
where
    E: Endpoint,
{
    match unsafe {
        libc::sendto(
            soc.0,
            buf.as_ptr().cast(),
            buf.len(),
            0,
            ep.as_ptr(),
            ep.len(),
        )
    } {
        0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
        -1 => Err(unsafe { OsError::last() }),
        len => Ok(len as usize),
    }
}

pub fn write(soc: &ConnectedSocket, buf: &[u8]) -> Result<usize> {
    match unsafe { libc::write(soc.0, buf.as_ptr().cast(), buf.len()) } {
        0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
        -1 => Err(unsafe { OsError::last() }),
        len => Ok(len as usize),
    }
}

pub fn wait_for_readable(soc: &ConnectedSocket, timeout: Timeout) -> Result<()> {
    let mut poll = libc::pollfd {
        fd: soc.0,
        events: libc::POLLIN,
        revents: 0,
    };
    match unsafe { libc::poll(&mut poll, 1, timeout.as_millis_i32()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Err(OsError::OPERATION_CANCELED),
        _ => Ok(()),
    }
}

pub fn wait_for_writable(soc: &ConnectedSocket, timeout: Timeout) -> Result<()> {
    let mut poll = libc::pollfd {
        fd: soc.0,
        events: libc::POLLOUT,
        revents: 0,
    };
    match unsafe { libc::poll(&mut poll, 1, timeout.as_millis_i32()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Err(OsError::OPERATION_CANCELED),
        _ => Ok(()),
    }
}

pub fn getsockname<E>(soc: &ConnectedSocket) -> Result<E>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
    match unsafe { libc::getsockname(soc.0, sa.as_mut_ptr().cast(), &mut salen) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(unsafe { E::init(sa, salen) }),
        _ => unreachable!(),
    }
}

pub fn getpeername<E>(soc: &ConnectedSocket) -> Result<E>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
    match unsafe { libc::getpeername(soc.0, sa.as_mut_ptr().cast(), &mut salen) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(unsafe { E::init(sa, salen) }),
        _ => unreachable!(),
    }
}

pub fn shutdown(soc: &ConnectedSocket, how: Shutdown) -> Result<()> {
    match unsafe { libc::shutdown(soc.0, how.into()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(()),
        _ => unreachable!(),
    }
}

pub trait SocketOption: Sized {
    const SIZE: SocklenType = mem::size_of::<Self>() as u32;

    fn as_ptr(&self) -> *const libc::c_void {
        self as *const _ as *const _
    }

    fn len(&self) -> SocklenType {
        mem::size_of_val(self) as SocklenType
    }

    unsafe fn init(data: MaybeUninit<Self>, len: SocklenType) -> Self {
        assert_eq!(len, Self::SIZE);
        data.assume_init()
    }
}

impl SocketOption for i32 {}

pub fn setsockopt<T>(soc: &ConnectedSocket, level: i32, name: i32, data: T) -> Result<()>
where
    T: SocketOption,
{
    match unsafe { libc::setsockopt(soc.0, level, name, data.as_ptr(), data.len()) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(()),
        _ => unreachable!(),
    }
}

pub fn getsockopt<T>(soc: &ConnectedSocket, level: i32, name: i32) -> Result<T>
where
    T: SocketOption,
{
    let mut data = MaybeUninit::<T>::uninit();
    let mut len = T::SIZE;
    match unsafe { libc::getsockopt(soc.0, level, name, data.as_mut_ptr().cast(), &mut len) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Ok(unsafe { T::init(data, len) }),
        _ => unreachable!(),
    }
}

pub fn signalfd(mask: &libc::sigset_t) -> Result<ConnectedSocket> {
    match unsafe { libc::signalfd(-1, mask, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC) } {
        -1 => Err(unsafe { OsError::last() }),
        sfd => Ok(ConnectedSocket(sfd)),
    }
}

pub fn signal_read(sfd: &ConnectedSocket) -> Result<Signal> {
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    const LEN: isize = mem::size_of::<libc::signalfd_siginfo>() as isize;
    match unsafe { libc::read(sfd.0, ssi.as_mut_ptr().cast(), mem::size_of_val(&ssi)) } {
        -1 => Err(unsafe { OsError::last() }),
        0 => Err(OsError::CONNECTION_ABORTED),
        LEN => {
            let ssi = unsafe { ssi.assume_init() };
            Ok(Signal {
                signo: ssi.ssi_signo,
            })
        }
        _ => unreachable!(),
    }
}

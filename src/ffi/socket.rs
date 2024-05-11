use super::{Signal, Timeout};
use crate::error::OsError;
use crate::socket_base::{Endpoint, Protocol, Shutdown, SocklenType};
use std::mem::{self, MaybeUninit};
use std::os::fd::RawFd;
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

    pub fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

pub trait IntoSocket {
    type Socket;

    fn into_socket(&self, soc: ConnectedSocket) -> Self::Socket;
}

fn close(soc: &ConnectedSocket) -> Result<()> {
    unsafe {
        match libc::close(soc.0) {
            -1 => Err(OsError::last()),
            0 => Ok(()),
            _ => unreachable!(),
        }
    }
}

pub fn socket<P>(pro: P) -> Result<ConnectedSocket>
where
    P: Protocol,
{
    let socktype: i32 = pro.socket_type().into();
    unsafe {
        match libc::socket(
            pro.family_type().into(),
            socktype | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            pro.protocol_type().into(),
        ) {
            -1 => Err(OsError::last()),
            soc => Ok(ConnectedSocket(soc)),
        }
    }
}

pub fn bind<E>(soc: &ConnectedSocket, ep: &E) -> Result<()>
where
    E: Endpoint,
{
    unsafe {
        match libc::bind(soc.0, ep.as_ptr(), ep.len()) {
            -1 => Err(OsError::last()),
            0 => Ok(()),
            _ => unreachable!(),
        }
    }
}

pub fn listen(soc: &ConnectedSocket, backlog: i32) -> Result<()> {
    unsafe {
        match libc::listen(soc.0, backlog) {
            -1 => Err(OsError::last()),
            0 => Ok(()),
            _ => unreachable!(),
        }
    }
}

pub fn connect<E>(soc: &ConnectedSocket, ep: &E) -> Result<()>
where
    E: Endpoint,
{
    unsafe {
        match libc::connect(soc.0, ep.as_ptr(), ep.len()) {
            -1 => Err(OsError::last()),
            0 => Ok(()),
            _ => unreachable!(),
        }
    }
}

pub fn accept<E>(soc: &ConnectedSocket) -> Result<(ConnectedSocket, E)>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
    unsafe {
        match libc::accept4(
            soc.0,
            sa.as_mut_ptr().cast(),
            &mut salen,
            libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
        ) {
            -1 => Err(OsError::last()),
            soc => Ok((ConnectedSocket(soc), E::init(sa, salen))),
        }
    }
}

pub fn read(soc: &ConnectedSocket, buf: &mut [u8]) -> Result<usize> {
    unsafe {
        match libc::read(soc.0, buf.as_mut_ptr().cast(), buf.len()) {
            0 => Err(OsError::CONNECTION_ABORTED),
            -1 => Err(OsError::last()),
            len => Ok(len as usize),
        }
    }
}

pub fn receive(soc: &ConnectedSocket, buf: &mut [u8]) -> Result<usize> {
    unsafe {
        match libc::recv(soc.0, buf.as_mut_ptr().cast(), buf.len(), 0) {
            0 => Err(OsError::CONNECTION_ABORTED),
            -1 => Err(OsError::last()),
            len => Ok(len as usize),
        }
    }
}

pub fn receive_from<E>(soc: &ConnectedSocket, buf: &mut [u8]) -> Result<(usize, E)>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
    unsafe {
        match libc::recvfrom(
            soc.0,
            buf.as_mut_ptr().cast(),
            buf.len(),
            0,
            sa.as_mut_ptr().cast(),
            &mut salen,
        ) {
            0 => Err(OsError::CONNECTION_ABORTED),
            -1 => Err(OsError::last()),
            len => Ok((len as usize, E::init(sa, salen))),
        }
    }
}

pub fn send(soc: &ConnectedSocket, buf: &[u8]) -> Result<usize> {
    unsafe {
        match libc::send(soc.0, buf.as_ptr().cast(), buf.len(), 0) {
            0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
            -1 => Err(OsError::last()),
            len => Ok(len as usize),
        }
    }
}

pub fn send_to<E>(soc: &ConnectedSocket, buf: &[u8], ep: &E) -> Result<usize>
where
    E: Endpoint,
{
    unsafe {
        match libc::sendto(
            soc.0,
            buf.as_ptr().cast(),
            buf.len(),
            0,
            ep.as_ptr(),
            ep.len(),
        ) {
            0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
            -1 => Err(OsError::last()),
            len => Ok(len as usize),
        }
    }
}

pub fn write(soc: &ConnectedSocket, buf: &[u8]) -> Result<usize> {
    unsafe {
        match libc::write(soc.0, buf.as_ptr().cast(), buf.len()) {
            0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
            -1 => Err(OsError::last()),
            len => Ok(len as usize),
        }
    }
}

pub fn wait_for_readable(soc: &ConnectedSocket, timeout: Timeout) -> Result<()> {
    let mut poll = libc::pollfd {
        fd: soc.0,
        events: libc::POLLIN,
        revents: 0,
    };
    unsafe {
        match libc::poll(&mut poll, 1, timeout.into_poll()) {
            -1 => Err(OsError::last()),
            0 => Err(OsError::OPERATION_CANCELED),
            _ => Ok(()),
        }
    }
}

pub fn wait_for_writable(soc: &ConnectedSocket, timeout: Timeout) -> Result<()> {
    let mut poll = libc::pollfd {
        fd: soc.0,
        events: libc::POLLOUT,
        revents: 0,
    };
    unsafe {
        match libc::poll(&mut poll, 1, timeout.into_poll()) {
            -1 => Err(OsError::last()),
            0 => Err(OsError::OPERATION_CANCELED),
            _ => Ok(()),
        }
    }
}

pub fn getsockname<E>(soc: &ConnectedSocket) -> Result<E>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
    unsafe {
        match libc::getsockname(soc.0, sa.as_mut_ptr().cast(), &mut salen) {
            -1 => Err(OsError::last()),
            0 => Ok(E::init(sa, salen)),
            _ => unreachable!(),
        }
    }
}

pub fn getpeername<E>(soc: &ConnectedSocket) -> Result<E>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
    unsafe {
        match libc::getpeername(soc.0, sa.as_mut_ptr().cast(), &mut salen) {
            -1 => Err(OsError::last()),
            0 => Ok(E::init(sa, salen)),
            _ => unreachable!(),
        }
    }
}

pub fn shutdown(soc: &ConnectedSocket, how: Shutdown) -> Result<()> {
    unsafe {
        match libc::shutdown(soc.0, how.into()) {
            -1 => Err(OsError::last()),
            0 => Ok(()),
            _ => unreachable!(),
        }
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
    unsafe {
        match libc::setsockopt(soc.0, level, name, data.as_ptr(), data.len()) {
            -1 => Err(OsError::last()),
            0 => Ok(()),
            _ => unreachable!(),
        }
    }
}

pub fn getsockopt<T>(soc: &ConnectedSocket, level: i32, name: i32) -> Result<T>
where
    T: SocketOption,
{
    let mut data = MaybeUninit::<T>::uninit();
    let mut len = T::SIZE;
    unsafe {
        match libc::getsockopt(soc.0, level, name, data.as_mut_ptr().cast(), &mut len) {
            -1 => Err(OsError::last()),
            0 => Ok(T::init(data, len)),
            _ => unreachable!(),
        }
    }
}

pub fn signalfd(mask: &libc::sigset_t) -> Result<ConnectedSocket> {
    unsafe {
        match libc::signalfd(-1, mask, libc::SFD_NONBLOCK | libc::SFD_CLOEXEC) {
            -1 => Err(OsError::last()),
            sfd => Ok(ConnectedSocket(sfd)),
        }
    }
}

pub fn signal_read(sfd: &ConnectedSocket) -> Result<Signal> {
    let mut ssi = MaybeUninit::<libc::signalfd_siginfo>::uninit();
    const LEN: isize = mem::size_of::<libc::signalfd_siginfo>() as isize;
    unsafe {
        match libc::read(sfd.0, ssi.as_mut_ptr().cast(), mem::size_of_val(&ssi)) {
            -1 => Err(OsError::last()),
            0 => Err(OsError::CONNECTION_ABORTED),
            LEN => Ok(Signal {
                signo: ssi.assume_init().ssi_signo,
            }),
            _ => unreachable!(),
        }
    }
}

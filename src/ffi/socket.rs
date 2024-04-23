use crate::socket_base::{Endpoint, Protocol, Shutdown, SocklenType};
use crate::error::{OsError, ResolverError};
use std::ffi::CStr;
use std::mem;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
use std::ptr;
use std::result;
use std::time::Duration;

type Result<T> = result::Result<T, OsError>;

pub fn socket<P>(pro: P) -> Result<OwnedFd>
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
            soc => Ok(OwnedFd::from_raw_fd(soc)),
        }
    }
}

pub fn bind<E>(soc: &OwnedFd, ep: &E) -> Result<()>
where
    E: Endpoint,
{
    unsafe {
        match libc::bind(soc.as_raw_fd(), ep.as_ptr(), ep.len()) {
            -1 => Err(OsError::last()),
            0 => Ok(()),
            _ => unreachable!(),
        }
    }
}

pub fn listen(soc: &OwnedFd, backlog: i32) -> Result<()> {
    unsafe {
        match libc::listen(soc.as_raw_fd(), backlog) {
            -1 => Err(OsError::last()),
            0 => Ok(()),
            _ => unreachable!(),
        }
    }
}

pub fn connect<E>(soc: &OwnedFd, ep: &E) -> Result<()>
where
    E: Endpoint,
{
    unsafe {
        match libc::connect(soc.as_raw_fd(), ep.as_ptr(), ep.len()) {
            -1 => Err(OsError::last()),
            0 => Ok(()),
            _ => unreachable!(),
        }
    }
}

pub fn accept<E>(soc: &OwnedFd) -> Result<(OwnedFd, E)>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
    unsafe {
        match libc::accept4(
            soc.as_raw_fd(),
            sa.as_mut_ptr().cast(),
            &mut salen,
            libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
        ) {
            -1 => Err(OsError::last()),
            soc => Ok((OwnedFd::from_raw_fd(soc), E::init(sa, salen))),
        }
    }
}

pub fn read(soc: &OwnedFd, buf: &mut [u8]) -> Result<usize> {
    unsafe {
        match libc::read(soc.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len()) {
            0 => Err(OsError::CONNECTION_ABORTED),
            -1 => Err(OsError::last()),
            len => Ok(len as usize),
        }
    }
}

pub fn receive(soc: &OwnedFd, buf: &mut [u8]) -> Result<usize> {
    unsafe {
        match libc::recv(soc.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len(), 0) {
            0 => Err(OsError::CONNECTION_ABORTED),
            -1 => Err(OsError::last()),
            len => Ok(len as usize),
        }
    }
}

pub fn receive_from<E>(soc: &OwnedFd, buf: &mut [u8]) -> Result<(usize, E)>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
    unsafe {
        match libc::recvfrom(
            soc.as_raw_fd(),
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

pub fn send(soc: &OwnedFd, buf: &[u8]) -> Result<usize> {
    unsafe {
        match libc::send(soc.as_raw_fd(), buf.as_ptr().cast(), buf.len(), 0) {
            0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
            -1 => Err(OsError::last()),
            len => Ok(len as usize),
        }
    }
}

pub fn send_to<E>(soc: &OwnedFd, buf: &[u8], ep: &E) -> Result<usize>
where
    E: Endpoint,
{
    unsafe {
        match libc::sendto(
            soc.as_raw_fd(),
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

pub fn write(soc: &OwnedFd, buf: &[u8]) -> Result<usize> {
    unsafe {
        match libc::write(soc.as_raw_fd(), buf.as_ptr().cast(), buf.len()) {
            0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
            -1 => Err(OsError::last()),
            len => Ok(len as usize),
        }
    }
}

fn into_i32_millis(timeout: Duration) -> i32 {
    // i32::MAX  = 2_147_483_647
    //          <= 2_147_482_000 + 999.000_000
    let secs = timeout.as_secs();
    if secs >= 2_148_483_000 {
        -1
    } else {
        let millis = (secs / 1000) as i32;
        millis + (timeout.subsec_nanos() / 1_000_000) as i32
    }
}

pub fn wait_for_readable(soc: &OwnedFd, timeout: Duration) -> Result<()> {
    let mut poll = libc::pollfd {
        fd: soc.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    unsafe {
        match libc::poll(&mut poll, 1, into_i32_millis(timeout)) {
            -1 => Err(OsError::last()),
            0 => Err(OsError::OPERATION_CANCELED),
            _ => Ok(()),
        }
    }
}

pub fn wait_for_writable(soc: &OwnedFd, timeout: Duration) -> Result<()> {
    let mut poll = libc::pollfd {
        fd: soc.as_raw_fd(),
        events: libc::POLLOUT,
        revents: 0,
    };
    unsafe {
        match libc::poll(&mut poll, 1, into_i32_millis(timeout)) {
            -1 => Err(OsError::last()),
            0 => Err(OsError::OPERATION_CANCELED),
            _ => Ok(()),
        }
    }
}

pub fn getsockname<E>(soc: &OwnedFd) -> Result<E>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
    unsafe {
        match libc::getsockname(soc.as_raw_fd(), sa.as_mut_ptr().cast(), &mut salen) {
            -1 => Err(OsError::last()),
            0 => Ok(E::init(sa, salen)),
            _ => unreachable!(),
        }
    }
}

pub fn getpeername<E>(soc: &OwnedFd) -> Result<E>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
    unsafe {
        match libc::getpeername(soc.as_raw_fd(), sa.as_mut_ptr().cast(), &mut salen) {
            -1 => Err(OsError::last()),
            0 => Ok(E::init(sa, salen)),
            _ => unreachable!(),
        }
    }
}

pub fn shutdown(soc: &OwnedFd, how: Shutdown) -> Result<()> {
    unsafe {
        match libc::shutdown(soc.as_raw_fd(), how.into()) {
            -1 => Err(OsError::last()),
            0 => Ok(()),
            _ => unreachable!(),
        }
    }
}

pub fn close(soc: OwnedFd) -> Result<()> {
    unsafe {
        match libc::close(soc.into_raw_fd()) {
            -1 => Err(OsError::last()),
            0 => Ok(()),
            _ => unreachable!(),
        }
    }
}

pub fn getaddrinfo(
    node: &CStr,
    serv: &CStr,
    hints: libc::addrinfo,
) -> result::Result<*mut libc::addrinfo, ResolverError> {
    let mut base = MaybeUninit::<*mut libc::addrinfo>::uninit();
    unsafe {
        let node = if node.is_empty() {
            ptr::null()
        } else {
            node.as_ptr()
        };
        let serv = if serv.is_empty() {
            ptr::null()
        } else {
            serv.as_ptr()
        };
        match libc::getaddrinfo(node, serv, &hints, base.as_mut_ptr()) {
            0 => Ok(base.assume_init()),
            err => Err(ResolverError::from_raw(err)),
        }
    }
}

pub fn freeaddrinfo(ai: *mut libc::addrinfo) {
    unsafe { libc::freeaddrinfo(ai) }
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

pub fn setsockopt<T>(soc: &OwnedFd, level: i32, name: i32, data: T) -> Result<()>
where
    T: SocketOption,
{
    unsafe {
        match libc::setsockopt(soc.as_raw_fd(), level, name, data.as_ptr(), data.len()) {
            -1 => Err(OsError::last()),
            0 => Ok(()),
            _ => unreachable!(),
        }
    }
}

pub fn getsockopt<T>(soc: &OwnedFd, level: i32, name: i32) -> Result<T>
where
    T: SocketOption,
{
    let mut data = MaybeUninit::<T>::uninit();
    let mut len = T::SIZE;
    unsafe {
        match libc::getsockopt(
            soc.as_raw_fd(),
            level,
            name,
            data.as_mut_ptr().cast(),
            &mut len,
        ) {
            -1 => Err(OsError::last()),
            0 => Ok(T::init(data, len)),
            _ => unreachable!(),
        }
    }
}

use crate::{Endpoint, OsError, Protocol, ResolverError, Shutdown, SocklenType};
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
use std::ptr;
use std::mem;
use std::result;
use std::time::Duration;

type Result<T> = result::Result<T, OsError>;

pub fn socket<P>(pro: P) -> Result<OwnedFd>
where
    P: Protocol,
{
    let socktype: i32 = pro.socket_type().into();
    unsafe {
        let soc = libc::socket(
            pro.family_type().into(),
            socktype | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            pro.protocol_type().into(),
        );
        if soc < 0 {
            return Err(OsError::last());
        }
        Ok(OwnedFd::from_raw_fd(soc))
    }
}

pub fn bind<E>(soc: &OwnedFd, ep: &E) -> Result<()>
where
    E: Endpoint,
{
    unsafe {
        let err = libc::bind(soc.as_raw_fd(), ep.as_ptr(), ep.len());
        if err < 0 {
            return Err(OsError::last());
        }
        Ok(())
    }
}

pub fn listen(soc: &OwnedFd, backlog: i32) -> Result<()> {
    unsafe {
        let err = libc::listen(soc.as_raw_fd(), backlog);
        if err < 0 {
            return Err(OsError::last());
        }
        Ok(())
    }
}

pub fn connect<E>(soc: &OwnedFd, ep: &E) -> Result<()>
where
    E: Endpoint,
{
    unsafe {
        let err = libc::connect(soc.as_raw_fd(), ep.as_ptr(), ep.len());
        if err < 0 {
            return Err(OsError::last());
        }
        Ok(())
    }
}

pub fn accept<E>(soc: &OwnedFd) -> Result<(OwnedFd, E)>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
    unsafe {
        let soc = libc::accept(soc.as_raw_fd(), sa.as_mut_ptr().cast(), &mut salen);
        if soc < 0 {
            return Err(OsError::last());
        }
        let soc = OwnedFd::from_raw_fd(soc);
        Ok((soc, E::init(sa, salen)))
    }
}

pub fn write(soc: &OwnedFd, buf: &[u8]) -> Result<usize> {
    unsafe {
        let len = libc::write(soc.as_raw_fd(), buf.as_ptr().cast(), buf.len());
        if len < 0 {
            return Err(OsError::last());
        }
        Ok(len as usize)
    }
}

pub fn send(soc: &OwnedFd, buf: &[u8]) -> Result<usize> {
    unsafe {
        let len = libc::send(soc.as_raw_fd(), buf.as_ptr().cast(), buf.len(), 0);
        if len < 0 {
            return Err(OsError::last());
        }
        Ok(len as usize)
    }
}

pub fn send_to<E>(soc: &OwnedFd, buf: &[u8], ep: &E) -> Result<usize>
where
    E: Endpoint,
{
    unsafe {
        let len = libc::sendto(
            soc.as_raw_fd(),
            buf.as_ptr() as *const libc::c_void,
            buf.len(),
            0,
            ep.as_ptr(),
            ep.len(),
        );
        if len < 0 {
            return Err(OsError::last());
        }
        Ok(len as usize)
    }
}

pub fn read(soc: &OwnedFd, buf: &mut [u8]) -> Result<usize> {
    unsafe {
        let len = libc::read(soc.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len());
        if len < 0 {
            return Err(OsError::last());
        }
        Ok(len as usize)
    }
}

pub fn receive(soc: &OwnedFd, buf: &mut [u8]) -> Result<usize> {
    unsafe {
        let len = libc::recv(soc.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len(), 0);
        if len < 0 {
            return Err(OsError::last());
        }
        Ok(len as usize)
    }
}

pub fn receive_from<E>(soc: &OwnedFd, buf: &mut [u8]) -> Result<(usize, E)>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
    unsafe {
        let len = libc::recvfrom(
            soc.as_raw_fd(),
            buf.as_mut_ptr() as *mut libc::c_void,
            buf.len(),
            0,
            sa.as_mut_ptr().cast(),
            &mut salen,
        );
        if len < 0 {
            return Err(OsError::last());
        }
        Ok((len as usize, E::init(sa, salen)))
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

pub fn wait_readable(soc: &OwnedFd, timeout: Duration) -> Result<()> {
    let mut poll = libc::pollfd {
        fd: soc.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    unsafe {
        let err = libc::poll(&mut poll, 1, into_i32_millis(timeout));
        match err {
            -1 => Err(OsError::last()),
            0 => Err(OsError::OPERATION_CANCELED),
            _ => Ok(()),
        }
    }
}

pub fn wait_writable(soc: &OwnedFd, timeout: Duration) -> Result<()> {
    let mut poll = libc::pollfd {
        fd: soc.as_raw_fd(),
        events: libc::POLLOUT,
        revents: 0,
    };
    unsafe {
        let err = libc::poll(&mut poll, 1, into_i32_millis(timeout));
        match err {
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
        let err = libc::getsockname(soc.as_raw_fd(), sa.as_mut_ptr().cast(), &mut salen);
        if err < 0 {
            return Err(OsError::last());
        }
        Ok(E::init(sa, salen))
    }
}

pub fn getpeername<E>(soc: &OwnedFd) -> Result<E>
where
    E: Endpoint,
{
    let mut sa = MaybeUninit::<E>::uninit();
    let mut salen = E::SIZE;
    unsafe {
        let err = libc::getpeername(soc.as_raw_fd(), sa.as_mut_ptr().cast(), &mut salen);
        if err < 0 {
            return Err(OsError::last());
        }
        Ok(E::init(sa, salen))
    }
}

pub fn shutdown(soc: &OwnedFd, how: Shutdown) -> Result<()> {
    unsafe {
        let err = libc::shutdown(soc.as_raw_fd(), how.into());
        if err < 0 {
            return Err(OsError::last());
        }
        Ok(())
    }
}

pub fn close(soc: OwnedFd) -> Result<()> {
    unsafe {
        let err = libc::close(soc.into_raw_fd());
        if err < 0 {
            return Err(OsError::last());
        }
        Ok(())
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
        let err = libc::getaddrinfo(node, serv, &hints, base.as_mut_ptr());
        if err != 0 {
            return Err(ResolverError::from_raw(err));
        }
        Ok(base.assume_init())
    }
}

pub fn freeaddrinfo(ai: *mut libc::addrinfo) {
    unsafe { libc::freeaddrinfo(ai) }
}


pub trait SocketOption : Sized {
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
        let err = libc::setsockopt(
            soc.as_raw_fd(),
            level,
            name,
            data.as_ptr(),
            data.len(),
        );
        if err < 0 {
            return Err(OsError::last())
        }
        Ok(())
    }
}


pub fn getsockopt<T>(soc: &OwnedFd, level: i32, name: i32) -> Result<T>
where
    T: SocketOption,
{
    let mut data = MaybeUninit::<T>::uninit();
    let mut len = T::SIZE;
    unsafe {
        let err = libc::getsockopt(
            soc.as_raw_fd(),
            level,
            name,
            data.as_mut_ptr().cast(),
            &mut len,
        );
        if err < 0 {
            return Err(OsError::last())
        }
        Ok(T::init(data, len))
    }
}

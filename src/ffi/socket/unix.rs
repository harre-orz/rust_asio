use crate::error::OsError;
use crate::socket_base::{Endpoint, Protocol, Shutdown, SockAddr};
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr;
use std::time::Instant;

type Result<T> = std::result::Result<T, OsError>;

pub struct Fd(libc::c_int);

impl Drop for Fd {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.0);
        }
    }
}

impl Fd {
    pub unsafe fn new_unchecked(fd: libc::c_int) -> Self {
        Self(fd)
    }

    pub unsafe fn as_raw_fd(&self) -> libc::c_int {
        self.0
    }

    pub fn close(self) -> Result<()> {
        unsafe {
            match libc::close(self.0) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    #[cfg(target_os = "linux")]
    pub fn open(filename: &CStr) -> Result<Self> {
        unsafe {
            match libc::open(filename.as_ptr().cast(), libc::O_CLOEXEC | libc::O_NONBLOCK) {
                -1 => Err(OsError::last()),
                soc => Ok(Self(soc)),
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub fn open(filename: &CStr) -> Result<Self> {
        unsafe {
            match libc::open(filename.as_ptr().cast(), 0) {
                -1 => Err(OsError::last()),
                fd => {
                    let fd = Self(fd);
                    // soc.set_cloexec()?;
                    // soc.set_nonblock()?;
                    Ok(fd)
                }
            }
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match libc::read(self.0, buf.as_mut_ptr().cast(), buf.len()) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn read_data<T>(&self) -> Result<T> {
        let mut buf = MaybeUninit::<T>::uninit();
        unsafe {
            match libc::read(self.0, buf.as_mut_ptr().cast(), size_of::<T>()) {
                -1 => Err(OsError::last()),
                len if len == size_of::<T>() as isize => Ok(buf.assume_init()),
                _ => panic!(),
            }
        }
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        unsafe {
            match libc::write(self.0, buf.as_ptr().cast(), buf.len()) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    #[allow(dead_code)]
    fn set_cloexec(&mut self) -> Result<()> {
        unsafe {
            match libc::fcntl(self.0, libc::F_SETFD, libc::FD_CLOEXEC) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    #[allow(dead_code)]
    fn set_nonblock(&mut self) -> Result<()> {
        unsafe {
            match libc::fcntl(self.0, libc::F_SETFL, libc::O_NONBLOCK) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }
}

fn into_poll(time: Option<Instant>) -> i32 {
    if let Some(time) = time {
        let time = time.duration_since(Instant::now()).as_millis();
        if time > i32::MAX as u128 {
            -1
        } else {
            time as i32
        }
    } else {
        -1
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

        unsafe { data.assume_init() }
    }
}

impl SocketOption for i32 {}

fn setsockopt<T>(soc: &Socket, level: i32, name: i32, data: T) -> Result<()>
where
    T: SocketOption,
{
    unsafe {
        match libc::setsockopt(soc.0.0, level, name, data.as_ptr(), data.len()) {
            -1 => Err(OsError::last()),
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
        match libc::getsockopt(soc.0.0, level, name, data.as_mut_ptr().cast(), &mut len) {
            -1 => Err(OsError::last()),
            _ => Ok(T::init(data, len)),
        }
    }
}

pub struct Socket(Fd);

impl Socket {
    pub unsafe fn from_raw_fd(fd: Fd) -> Self {
        Socket(fd)
    }

    #[cfg(target_os = "linux")]
    pub fn new<P>(pro: P) -> Result<Self>
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
                soc => Ok(Socket(Fd::new_unchecked(soc))),
            }
        }
    }

    #[cfg(target_os = "macos")]
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
                -1 => Err(OsError::last()),
                fd => {
                    let mut fd = Fd::new_unchecked(fd);
                    fd.set_cloexec()?;
                    fd.set_nonblock()?;
                    Ok(Socket(fd))
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    pub fn socketpair<P>(pro: P) -> Result<(Socket, Socket)>
    where
        P: Protocol,
    {
        let mut sv = MaybeUninit::<[libc::c_int; 2]>::uninit();
        let socktype: i32 = pro.socket_type().into();
        unsafe {
            match libc::socketpair(
                pro.family_type().into(),
                socktype | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                pro.protocol_type().into(),
                sv.as_mut_ptr().cast(),
            ) {
                -1 => Err(OsError::last()),
                0 => {
                    let sv = sv.assume_init();
                    let s1 = Socket(Fd::new_unchecked(sv[0]));
                    let s2 = Socket(Fd::new_unchecked(sv[1]));
                    Ok((s1, s2))
                }
                _ => unreachable!(),
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub fn socketpair<P>(pro: P) -> Result<(Socket, Socket)>
    where
        P: Protocol,
    {
        let mut sv = MaybeUninit::<[libc::c_int; 2]>::uninit();
        unsafe {
            match libc::socketpair(
                pro.family_type().into(),
                pro.socket_type().into(),
                pro.protocol_type().into(),
                sv.as_mut_ptr().cast(),
            ) {
                -1 => Err(unsafe { OsError::last() }),
                _ => {
                    let sv = sv.assume_init();
                    let mut s1 = Fd::new_unchecked(sv[0]);
                    let mut s2 = Fd::new_unchecked(sv[1]);
                    s1.set_cloexec()?;
                    s2.set_cloexec()?;
                    s1.set_nonblock()?;
                    s2.set_nonblock()?;
                    Ok((Socket(s1), Socket(s2)))
                }
            }
        }
    }

    pub fn as_fd(&self) -> &Fd {
        &self.0
    }

    pub fn bind<E>(&self, ep: &E) -> Result<()>
    where
        E: Endpoint,
    {
        let sa = ep.sockaddr();
        unsafe {
            match libc::bind(self.0.0, sa.as_ptr(), sa.len()) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn listen(&self, backlog: i32) -> Result<()> {
        unsafe {
            match libc::listen(self.0.0, backlog) {
                -1 => Err(OsError::last()),
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
            match libc::connect(self.0.0, sa.as_ptr(), sa.len()) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn close(self) -> Result<()> {
        self.0.close()
    }

    #[cfg(target_os = "linux")]
    pub fn nb_accept<E>(&self) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut salen = E::SockAddr::MAX_SIZE;
        unsafe {
            match libc::accept4(
                self.0.0,
                sa.as_mut_ptr().cast(),
                &mut salen,
                libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            ) {
                -1 => Err(OsError::last()),
                soc => {
                    let soc = Socket(Fd::new_unchecked(soc));
                    let sa = E::SockAddr::init(sa, salen);
                    Ok((soc, E::new(sa)))
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    pub fn nb_accept<E>(&self) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut salen = E::SockAddr::MAX_SIZE;
        unsafe {
            match libc::accept(self.0.0, sa.as_mut_ptr().cast(), &mut salen) {
                -1 => Err(OsError::last()),
                fd => {
                    let mut fd = Fd::new_unchecked(fd);
                    fd.set_cloexec()?;
                    fd.set_nonblock()?;
                    let sa = unsafe { E::SockAddr::init(sa, salen) };
                    Ok((Socket(fd), E::new(sa)))
                }
            }
        }
    }

    pub fn nb_receive(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match libc::recv(self.0.0, buf.as_mut_ptr().cast(), buf.len(), 0) {
                -1 => Err(OsError::last()),
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
            match libc::recvfrom(
                self.0.0,
                buf.as_mut_ptr().cast(),
                buf.len(),
                0,
                sa.as_mut_ptr().cast(),
                &mut salen,
            ) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => {
                    let sa = SockAddr::init(sa, salen);
                    Ok((len as usize, E::new(sa)))
                }
            }
        }
    }

    pub fn nb_send(&self, buf: &[u8]) -> Result<usize> {
        unsafe {
            match libc::send(self.0.0, buf.as_ptr().cast(), buf.len(), 0) {
                -1 => Err(OsError::last()),
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
            match libc::sendto(
                self.0.0,
                buf.as_ptr().cast(),
                buf.len(),
                0,
                sa.as_ptr(),
                sa.len(),
            ) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    pub fn wait_for_readable(&self, cto: Option<Instant>) -> Result<()> {
        let mut poll = libc::pollfd {
            fd: self.0.0,
            events: libc::POLLIN,
            revents: 0,
        };
        unsafe {
            match libc::poll(&mut poll, 1, into_poll(cto)) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::OPERATION_CANCELED),
                _ => Ok(()),
            }
        }
    }

    pub fn wait_for_writable(&self, cto: Option<Instant>) -> Result<()> {
        let mut poll = libc::pollfd {
            fd: self.0.0,
            events: libc::POLLOUT,
            revents: 0,
        };
        unsafe {
            match libc::poll(&mut poll, 1, into_poll(cto)) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::OPERATION_CANCELED),
                _ => Ok(()),
            }
        }
    }

    pub fn nb_read(&self, buf: &mut [u8]) -> Result<usize> {
        self.0.read(buf)
    }

    pub fn nb_write(&self, buf: &[u8]) -> Result<usize> {
        self.0.write(buf)
    }

    pub fn getsockname<E>(&self) -> Result<E>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut salen = E::SockAddr::MAX_SIZE;
        unsafe {
            match libc::getsockname(self.0.0, sa.as_mut_ptr().cast(), &mut salen) {
                -1 => Err(OsError::last()),
                _ => Ok(E::new(SockAddr::init(sa, salen))),
            }
        }
    }

    pub fn getpeername<E>(&self) -> Result<E>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut salen = E::SockAddr::MAX_SIZE;
        unsafe {
            match libc::getpeername(self.0.0, sa.as_mut_ptr().cast(), &mut salen) {
                -1 => Err(OsError::last()),
                _ => Ok(E::new(SockAddr::init(sa, salen))),
            }
        }
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        unsafe {
            match libc::shutdown(self.0.0, how.into()) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn reuse_addr(&self, on: bool) -> Result<()> {
        let on = if on { 1i32 } else { 0i32 };
        setsockopt(self, libc::SOL_SOCKET, libc::SO_REUSEADDR, on)?;
        Ok(())
    }
}

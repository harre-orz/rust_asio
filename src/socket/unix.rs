use crate::buffer::MsgBuf;
use crate::error::{OsError, Result};
use crate::sockaddr::{SockAddr, SockLen};
use crate::socket_base::{Endpoint, EndpointRef, GetSockOpt, Protocol, SetSockOpt};
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::time::Duration;
use std::{mem, ptr};

pub const MAX_CONNECTIONS: i32 = libc::SOMAXCONN;

/// Possible values which can be passed to the shutdown method.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(i32)]
pub enum Shutdown {
    /// Indicates that the reading portion of this socket should be shut down.
    Read = libc::SHUT_RD,

    /// Indicates that the writing portion of this socket should be shut down.
    Write = libc::SHUT_WR,

    /// Shut down both the reading and writing portions of this socket.
    Both = libc::SHUT_RDWR,
}

pub struct SocketType(i32);

impl SocketType {
    pub const SOCK_STREAM: Self = Self(libc::SOCK_STREAM);
    pub const SOCK_DGRAM: Self = Self(libc::SOCK_DGRAM);
    pub const SOCK_RAW: Self = Self(libc::SOCK_RAW);
    pub const SOCK_SEQPACKET: Self = Self(libc::SOCK_SEQPACKET);
}

impl Into<i32> for SocketType {
    fn into(self) -> i32 {
        self.0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Timeout(libc::c_int);

impl Timeout {
    pub const fn infinite() -> Self {
        Self(-1)
    }

    pub const fn from_duration(timeout: Duration) -> Self {
        let time = timeout.as_millis();
        if time > i32::MAX as u128 {
            Timeout::infinite()
        } else {
            Timeout(time as i32)
        }
    }

    pub const fn into_duration(self) -> Duration {
        let millis = if self.0 == -1 {
            u32::MAX
        } else {
            self.0 as u32
        };
        Duration::from_millis(millis as u64)
    }
}

pub(crate) struct Fd(libc::c_int);

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

    fn open_impl(filename: &CStr) -> Result<Self> {
        unsafe {
            match libc::open(filename.as_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) {
                -1 => Err(OsError::last()),
                soc => Ok(Self(soc)),
            }
        }
    }

    pub fn open(filename: &CStr) -> Result<Self> {
        let fd = Self::open_impl(filename)?;
        #[cfg(target_os = "macos")]
        fd.set_cloexec_nonblock()?;
        Ok(fd)
    }

    #[cfg(target_os = "macos")]
    fn set_cloexec_nonblock(&self) -> Result<()> {
        unsafe {
            match libc::fcntl(self.0, libc::F_SETFD, libc::FD_CLOEXEC) {
                -1 => Err(OsError::last()),
                _ => match libc::fcntl(self.0, libc::F_SETFL, libc::O_NONBLOCK) {
                    -1 => Err(OsError::last()),
                    _ => Ok(()),
                }
            }
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match libc::read(self.0, buf.as_mut_ptr().cast(), buf.len()) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
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
}

/// Low-level UNIX-based socket type.
pub struct Socket(Fd);

#[cfg(doc)]
impl Drop for Socket {
    fn drop(&mut self) {}
}

impl Socket {
    pub(crate) unsafe fn from_raw_fd(fd: Fd) -> Self {
        Socket(fd)
    }

    fn new_impl<P>(pro: P) -> Result<Self>
    where
        P: Protocol,
    {
        let socktype: i32 = pro.socket_type().into();
        #[cfg(target_os = "linux")]
        let socktype = socktype | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK;
        unsafe {
            match libc::socket(
                pro.family_type().into(),
                socktype,
                pro.protocol_type().into(),
            ) {
                -1 => Err(OsError::last()),
                soc => Ok(Socket(Fd::new_unchecked(soc))),
            }
        }
    }

    pub fn new<P>(pro: P) -> Result<Self>
    where
        P: Protocol,
    {
        let soc = Self::new_impl(pro)?;
        #[cfg(target_os = "macos")]
        soc.0.set_cloexec_nonblock()?;
        Ok(soc)
    }

    fn socketpair_impl<P>(pro: P) -> Result<(Socket, Socket)>
    where
        P: Protocol,
    {
        let mut sv: [MaybeUninit<libc::c_int>; 2] = [const { MaybeUninit::uninit() }; 2];
        let socktype: i32 = pro.socket_type().into();
        #[cfg(target_os = "linux")]
        let socktype = socktype | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK;
        unsafe {
            match libc::socketpair(
                pro.family_type().into(),
                socktype,
                pro.protocol_type().into(),
                sv[0].as_mut_ptr(),
            ) {
                -1 => Err(OsError::last()),
                _ => {
                    let sv = mem::transmute::<_, [libc::c_int; 2]>(sv);
                    let s1 = Socket(Fd::new_unchecked(sv[0]));
                    let s2 = Socket(Fd::new_unchecked(sv[1]));
                    Ok((s1, s2))
                }
            }
        }
    }

    pub fn socketpair<P>(pro: P) -> Result<(Socket, Socket)>
    where
        P: Protocol,
    {
        let (s1, s2) = Self::socketpair_impl(pro)?;
        #[cfg(target_os = "macos")]
        s1.0.set_cloexec_nonblock()?;
        #[cfg(target_os = "macos")]
        s2.0.set_cloexec_nonblock()?;
        Ok((s1, s2))
    }

    pub(crate) fn as_fd(&self) -> &Fd {
        &self.0
    }

    pub unsafe fn as_raw_fd(&self) -> libc::c_int {
        self.0.0
    }

    pub fn bind<E>(&self, ep: &EndpointRef<E>) -> Result<()>
    where
        E: Endpoint,
    {
        unsafe {
            match libc::bind(self.0.0, ep.sockaddr_ref().as_raw_ptr(), ep.sockaddr_len()) {
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

    pub fn connect<E>(&self, ep: &EndpointRef<E>) -> Result<()>
    where
        E: Endpoint,
    {
        unsafe {
            match libc::connect(self.0.0, ep.sockaddr_ref().as_raw_ptr(), ep.sockaddr_len()) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn close(self) -> Result<()> {
        let Fd(fd) = self.0;
        unsafe {
            match libc::close(fd) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn accept_impl<E>(&self) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match libc::accept4(
                self.0.0,
                sa.as_mut_ptr().cast(),
                &mut sa_len,
                libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            ) {
                -1 => Err(OsError::last()),
                soc => {
                    let soc = Socket(Fd::new_unchecked(soc));
                    let ep = E::from_sockaddr(E::SockAddr::init(sa, sa_len));
                    Ok((soc, ep))
                }
            }
        }
    }

    pub fn accept<E>(&self) -> Result<(Socket, E)>
    where
        E: Endpoint,
    {
        let (soc, ep) = self.accept_impl()?;
        #[cfg(target_os = "macos")]
        soc.0.set_cloexec_nonblock()?;
        Ok((soc, ep))
    }

    pub fn receive(&self, buf: &mut [u8]) -> Result<usize> {
        unsafe {
            match libc::recv(self.0.0, buf.as_mut_ptr().cast(), buf.len(), 0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
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
            match libc::recvfrom(
                self.0.0,
                buf.as_mut_ptr().cast(),
                buf.len(),
                0,
                sa.as_mut_ptr().cast(),
                &mut sa_len,
            ) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok((
                    len as usize,
                    E::from_sockaddr(E::SockAddr::init(sa, sa_len)),
                )),
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn receive_msg_impl(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        unsafe {
            match libc::recvmsg(self.0.0, mbuf.as_msghdr_ptr(), 0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len),
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn receive_msg_impl(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        if let Some(len) = mbuf.next() {
            Ok(len)
        } else {
            unsafe {
                mbuf.uninit();
                let mmsghdr = mbuf.as_mut_slice();
                match libc::recvmmsg(
                    self.0.0,
                    mmsghdr.as_mut_ptr(),
                    mmsghdr.len() as SockLen,
                    0,
                    ptr::null_mut(),
                ) {
                    -1 => Err(OsError::last()),
                    0 => Err(OsError::CONNECTION_ABORTED),
                    len => Ok(mbuf.set_len(len as usize)),
                }
            }
        }
    }

    pub fn receive_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.receive_msg_impl(mbuf)
    }

    pub fn send(&self, buf: &[u8]) -> Result<usize> {
        unsafe {
            match libc::send(self.0.0, buf.as_ptr().cast(), buf.len(), 0) {
                -1 => Err(OsError::last()),
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
            match libc::sendto(
                self.0.0,
                buf.as_ptr().cast(),
                buf.len(),
                0,
                ep.sockaddr_ref().as_raw_ptr(),
                ep.sockaddr_len() as SockLen,
            ) {
                -1 => Err(OsError::last()),
                0 if !buf.is_empty() => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len as usize),
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn send_msg_impl(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        unsafe {
            match libc::sendmsg(self.0.0, mbuf.as_msghdr_ptr(), 0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::CONNECTION_ABORTED),
                len => Ok(len),
            }
        }
    }
    #[cfg(target_os = "linux")]
    fn send_msg_impl(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        if let Some(len) = mbuf.next() {
            Ok(len)
        } else {
            unsafe {
                let mmsghdr = mbuf.as_mut_slice();
                match libc::sendmmsg(self.0.0, mmsghdr.as_mut_ptr(), mmsghdr.len() as SockLen, 0) {
                    -1 => Err(OsError::last()),
                    0 => Err(OsError::CONNECTION_ABORTED),
                    len => Ok(mbuf.set_len(len as usize)),
                }
            }
        }
    }

    pub fn send_msg(&self, mbuf: &mut MsgBuf) -> Result<usize> {
        self.send_msg_impl(mbuf)
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        self.0.read(buf)
    }

    pub fn write(&self, buf: &[u8]) -> Result<usize> {
        self.0.write(buf)
    }

    pub fn getsockname<E>(&self) -> Result<E>
    where
        E: Endpoint,
    {
        let mut sa = MaybeUninit::<E::SockAddr>::uninit();
        let mut sa_len = size_of::<E::SockAddr>() as SockLen;
        unsafe {
            match libc::getsockname(self.0.0, sa.as_mut_ptr().cast(), &mut sa_len) {
                -1 => Err(OsError::last()),
                _ => Ok(E::from_sockaddr(E::SockAddr::init(sa, sa_len))),
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
            match libc::getpeername(self.0.0, sa.as_mut_ptr().cast(), &mut sa_len) {
                -1 => Err(OsError::last()),
                _ => Ok(E::from_sockaddr(E::SockAddr::init(sa, sa_len))),
            }
        }
    }

    pub fn shutdown(&self, how: Shutdown) -> Result<()> {
        unsafe {
            match libc::shutdown(self.0.0, how as i32) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn setsockopt<P, S>(&self, pro: P, opt: &S) -> Result<()>
    where
        P: Protocol,
        S: SetSockOpt<P>,
    {
        let (level, name, data) = opt.data(pro);
        unsafe {
            match libc::setsockopt(
                self.0.0,
                level,
                name,
                data.as_ptr().cast(),
                data.len() as SockLen,
            ) {
                -1 => Err(OsError::last()),
                _ => Ok(()),
            }
        }
    }

    pub fn getsockopt<P, S>(&self, pro: P) -> Result<S>
    where
        P: Protocol,
        S: GetSockOpt<P>,
    {
        let (level, name, init) = S::init(pro);
        let mut data = MaybeUninit::<S>::uninit();
        let mut data_len = size_of::<S>() as SockLen;
        unsafe {
            match libc::getsockopt(
                self.0.0,
                level,
                name,
                data.as_mut_ptr().cast(),
                &mut data_len,
            ) {
                -1 => Err(OsError::last()),
                _ => Ok(init(data, data_len as usize)),
            }
        }
    }

    pub fn poll_in(&self, timeout: Timeout) -> Result<()> {
        let mut poll = libc::pollfd {
            fd: self.0.0,
            events: libc::POLLIN,
            revents: 0,
        };
        unsafe {
            match libc::poll(&mut poll, 1, timeout.0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::OPERATION_CANCELED),
                _ => Ok(()),
            }
        }
    }

    pub fn poll_out(&self, timeout: Timeout) -> Result<()> {
        let mut poll = libc::pollfd {
            fd: self.0.0,
            events: libc::POLLOUT,
            revents: 0,
        };
        unsafe {
            match libc::poll(&mut poll, 1, timeout.0) {
                -1 => Err(OsError::last()),
                0 => Err(OsError::OPERATION_CANCELED),
                _ => Ok(()),
            }
        }
    }
}

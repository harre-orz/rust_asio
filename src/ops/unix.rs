use std::io;
use std::mem;
use std::cmp;
use std::ptr;
use libc;
use backbone::Expiry;
use {Shutdown, Protocol, NonBlocking, AsSockAddr, IoControl, GetSocketOption, SetSocketOption};

pub use libc::{c_int, c_char, memcmp as c_memcmp};
pub use libc::{CLOCK_MONOTONIC, timeval, timespec};
pub use libc::{SHUT_RD, SHUT_WR, SHUT_RDWR};
pub use libc::{SOCK_DGRAM, SOCK_STREAM, SOCK_RAW};
pub use libc::{SOL_SOCKET, SO_REUSEADDR, SO_BROADCAST, SO_DEBUG, SO_DONTROUTE, SO_KEEPALIVE, SO_LINGER, SO_RCVBUF, SO_RCVLOWAT, SO_SNDBUF, SO_SNDLOWAT, SO_ACCEPTCONN};
pub use libc::{FIONREAD};
pub use libc::{IPPROTO_IP, IPPROTO_IPV6, IPV6_V6ONLY, };
pub use libc::{AF_INET, AF_INET6, IPPROTO_TCP};
pub use libc::{sockaddr_in, sockaddr_in6, sockaddr_un, sockaddr_storage};
pub use std::os::unix::io::{RawFd, AsRawFd};
pub const UNIX_PATH_MAX: usize = 108;
pub const SOCK_SEQPACKET: i32 = 5;
pub const AF_UNSPEC: i32 = 0;
pub const AF_LOCAL: i32 = 1;
pub const AI_PASSIVE: i32 = 0x0001;
pub const AI_NUMERICHOST: i32 = 0x0004;
pub const AI_NUMERICSERV: i32 = 0x0400;
pub const SIOCATMARK: i32  = 0x8905;
pub const IPPROTO_ICMP: i32 = 1;
pub const IPPROTO_ICMPV6: i32 = 58;

macro_rules! libc_try {
    ($expr:expr) => (match unsafe { $expr } {
        rc if rc >= 0 => rc,
        _ => return Err(::std::io::Error::last_os_error()),
    })
}

extern {
    #[cfg_attr(target_os = "linux", link_name = "__errno_location")]
    fn errno_location() -> *mut c_int;
}

pub fn errno() -> i32 {
    unsafe { *errno_location() }
}

pub enum AsyncResult<T> {
    Ok(T),
    Err(io::Error),
    WouldBlock,
}

// file descriptor operations.

pub fn close<F: AsRawFd>(fd: &F) -> io::Result<()> {
    libc_try!(libc::close(fd.as_raw_fd()));
    Ok(())
}

pub fn read<F: AsRawFd>(fd: &F, buf: &mut [u8]) -> io::Result<usize> {
    let size = libc_try!(libc::read(fd.as_raw_fd(), mem::transmute(buf.as_mut_ptr()), buf.len()));
    if size == 0 {
        Err(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
    } else {
        Ok(size as usize)
    }
}

pub fn read_with_nonblock<S: NonBlocking>(soc: &S, buf: &mut [u8]) -> AsyncResult<usize> {
    if let Err(err) = soc.set_native_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    match unsafe { libc::read(soc.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, buf.len()) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        0 => AsyncResult::Err(io::Error::new(io::ErrorKind::UnexpectedEof, "")),
        size => AsyncResult::Ok(size as usize),
    }
}

pub fn write<F: AsRawFd>(fd: &F, buf: &[u8]) -> io::Result<usize> {
    let size = libc_try!(libc::write(fd.as_raw_fd(), mem::transmute(buf.as_ptr()), buf.len()));
    if size == 0 {
        Err(io::Error::new(io::ErrorKind::WriteZero, ""))
    } else {
        Ok(size as usize)
    }
}

pub fn write_with_nonblock<S: NonBlocking>(soc: &S, buf: &[u8]) -> AsyncResult<usize> {
    if let Err(err) = soc.set_native_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    match unsafe { libc::write(soc.as_raw_fd(), buf.as_ptr() as *const libc::c_void, buf.len()) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        0 => AsyncResult::Err(io::Error::new(io::ErrorKind::WriteZero, "")),
        size => AsyncResult::Ok(size as usize),
    }
}

pub fn ioctl<F: AsRawFd, P: Protocol, T: IoControl<P>>(fd: &F, cmd: &mut T) -> io::Result<()> {
    libc_try!(libc::ioctl(fd.as_raw_fd(), cmd.name() as u64, cmd.data()));
    Ok(())
}

pub fn getflags<F: AsRawFd>(fd: &F) -> io::Result<i32> {
    Ok(libc_try!(libc::fcntl(fd.as_raw_fd(), libc::F_GETFL)))
}

pub fn setflags<F: AsRawFd>(fd: &F, flags: i32) -> io::Result<()> {
    libc_try!(libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags));
    Ok(())
}

pub fn getnonblock<F: AsRawFd>(fd: &F) -> io::Result<bool> {
    Ok((try!(getflags(fd)) & libc::O_NONBLOCK) != 0)
}

pub fn setnonblock<F: AsRawFd>(fd: &F, on: bool) -> io::Result<()> {
    let flags = try!(getflags(fd));
    setflags(fd, if on { flags | libc::O_NONBLOCK } else { flags & !libc::O_NONBLOCK })
}


// socket descriptor operations.

pub fn socket<P: Protocol>(pro: P) -> io::Result<RawFd> {
    Ok(libc_try!(libc::socket(
        pro.family_type(),
        pro.socket_type() | libc::SOCK_CLOEXEC,
        pro.protocol_type()
    )))
}

pub fn shutdown<S: AsRawFd>(soc: &S, how: Shutdown) -> io::Result<()> {
    libc_try!(libc::shutdown(soc.as_raw_fd(), how as i32));
    Ok(())
}

#[test]
fn test_enum_shutdown() {
    assert!(Shutdown::Read as i32 == SHUT_RD);
    assert!(Shutdown::Write as i32 == SHUT_WR);
    assert!(Shutdown::Both as i32 == SHUT_RDWR);
}

pub fn bind<S: AsRawFd, E: AsSockAddr>(soc: &S, ep: &E) -> io::Result<()> {
    libc_try!(libc::bind(soc.as_raw_fd(), mem::transmute(ep.as_sockaddr()), ep.size() as libc::socklen_t));
    Ok(())
}

pub fn connect<S: AsRawFd, E: AsSockAddr>(soc: &S, ep: &E) -> io::Result<()> {
    libc_try!(libc::connect(soc.as_raw_fd(), mem::transmute(ep.as_sockaddr()), ep.size() as libc::socklen_t));
    Ok(())
}

pub fn connect_with_nonblock<S, E>(soc: &S, ep: &E) -> AsyncResult<()>
    where S: NonBlocking,
          E: AsSockAddr,
{
    if let Err(err) = soc.set_native_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    match unsafe { libc::connect(soc.as_raw_fd(), mem::transmute(ep.as_sockaddr()), ep.size() as libc::socklen_t) } {
        0 => AsyncResult::Ok(()),
        _ => {
            let err = errno();
            if err == libc::EINPROGRESS {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
    }
}

pub const SOMAXCONN: u32 = 126;
pub fn listen<S: AsRawFd>(soc: &S, backlog: u32) -> io::Result<()> {
    libc_try!(libc::listen(soc.as_raw_fd(), cmp::min(backlog, SOMAXCONN) as i32));
    Ok(())
}

pub fn accept<S: AsRawFd, E: AsSockAddr>(soc: &S, mut ep: E) -> io::Result<(RawFd, E)> {
    let mut socklen = ep.capacity() as libc::socklen_t;
    let acc = libc_try!(libc::accept(soc.as_raw_fd(), mem::transmute(ep.as_mut_sockaddr()), &mut socklen));
    ep.resize(socklen as usize);
    Ok((acc, ep))
}

pub fn accept_with_nonblock<S, E>(soc: &S, ep: &mut E) -> AsyncResult<RawFd>
    where S: NonBlocking,
          E: AsSockAddr,
{
    if let Err(err) = setnonblock(soc, true) {
        return AsyncResult::Err(err);
    }
    let mut socklen = ep.capacity() as libc::socklen_t;
    match unsafe { libc::accept(soc.as_raw_fd(), mem::transmute(ep.as_mut_sockaddr()), &mut socklen) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        fd => {
            ep.resize(socklen as usize);
            AsyncResult::Ok(fd)
        },
    }
}

pub fn getsockname<S: AsRawFd, E: AsSockAddr>(soc: &S, mut ep: E) -> io::Result<E> {
    let mut socklen = ep.capacity() as libc::socklen_t;
    libc_try!(libc::getsockname(soc.as_raw_fd(), mem::transmute(ep.as_mut_sockaddr()), &mut socklen));
    ep.resize(socklen as usize);
    Ok(ep)
}

pub fn getpeername<S: AsRawFd, E: AsSockAddr>(soc: &S, mut ep: E) -> io::Result<E> {
    let mut socklen = ep.capacity() as libc::socklen_t;
    libc_try!(libc::getpeername(soc.as_raw_fd(), mem::transmute(ep.as_mut_sockaddr()), &mut socklen));
    ep.resize(socklen as usize);
    Ok(ep)
}

pub fn getsockopt<S: AsRawFd, P: Protocol, T: GetSocketOption<P>>(soc: &S) -> io::Result<T> {
    let mut cmd = T::default();
    let mut datalen = 0;
    libc_try!(libc::getsockopt(soc.as_raw_fd(), cmd.level(), cmd.name(), mem::transmute(cmd.data_mut()), &mut datalen));
    cmd.resize(datalen as usize);
    Ok(cmd)
}

pub fn setsockopt<S: AsRawFd, P: Protocol, T: SetSocketOption<P>>(soc: &S, cmd: T) -> io::Result<()> {
    libc_try!(libc::setsockopt(soc.as_raw_fd(), cmd.level(), cmd.name(), mem::transmute(cmd.data()), cmd.size() as libc::socklen_t));
    Ok(())
}

pub fn recv<S: AsRawFd>(soc: &S, buf: &mut [u8], flags: i32) -> io::Result<usize> {
    let size = libc_try!(libc::recv(soc.as_raw_fd(),buf.as_mut_ptr() as *mut libc::c_void, buf.len(), flags));
    Ok(size as usize)
}

pub fn recv_with_nonblock<S: NonBlocking>(soc: &S, buf: &mut [u8], flags: i32) -> AsyncResult<usize> {
    if let Err(err) = soc.set_native_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    match unsafe { libc::recv(soc.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, buf.len(), flags) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        0 => AsyncResult::Err(io::Error::new(io::ErrorKind::UnexpectedEof, "")),
        size => AsyncResult::Ok(size as usize),
    }
}

pub fn recvfrom<S: AsRawFd, E: AsSockAddr>(soc: &S, buf: &mut [u8], flags: i32, mut ep: E) -> io::Result<(usize, E)> {
    let mut socklen = ep.capacity() as libc::socklen_t;
    let size = libc_try!(libc::recvfrom(soc.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, buf.len(), flags, mem::transmute(ep.as_mut_sockaddr()), &mut socklen));
    ep.resize(socklen as usize);
    Ok((size as usize, ep))
}

pub fn recvfrom_with_nonblock<S: NonBlocking, E: AsSockAddr>(soc: &S, buf: &mut [u8], flags: i32, ep: &mut E) -> AsyncResult<usize> {
    if let Err(err) = soc.set_native_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    let mut socklen = ep.capacity() as libc::socklen_t;
    match unsafe { libc::recvfrom(soc.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, buf.len(), flags, mem::transmute(ep.as_mut_sockaddr()), &mut socklen) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        0 => {
            ep.resize(socklen as usize);
            AsyncResult::Err(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
        },
        size => {
            ep.resize(socklen as usize);
            AsyncResult::Ok(size as usize)
        }
    }
}

pub fn send<S: AsRawFd>(soc: &S, buf: &[u8], flags: i32) -> io::Result<usize> {
    let size = libc_try!(libc::send(soc.as_raw_fd(), buf.as_ptr() as *const libc::c_void, buf.len(), flags));
    Ok(size as usize)
}

pub fn send_with_nonblock<S: NonBlocking>(soc: &S, buf: &[u8], flags: i32) -> AsyncResult<usize> {
    if let Err(err) = soc.set_native_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    match unsafe { libc::send(soc.as_raw_fd(), buf.as_ptr() as *const libc::c_void, buf.len(), flags) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        0 => AsyncResult::Err(io::Error::new(io::ErrorKind::WriteZero, "")),
        size => AsyncResult::Ok(size as usize),
    }
}

pub fn sendto<S: AsRawFd, E: AsSockAddr>(soc: &S, buf: &[u8], flags: i32, ep: &E) -> io::Result<usize> {
    let size = libc_try!(libc::sendto(soc.as_raw_fd(), buf.as_ptr() as *const libc::c_void, buf.len(), flags, mem::transmute(ep.as_sockaddr()), ep.size() as libc::socklen_t));
    Ok(size as usize)
}

pub fn sendto_with_nonblock<S: NonBlocking, E: AsSockAddr>(soc: &S, buf: &[u8], flags: i32, ep: &E) -> AsyncResult<usize>
{
    if let Err(err) = soc.set_native_non_blocking(true) {
        return AsyncResult::Err(err);
    }
    match unsafe { libc::sendto(soc.as_raw_fd(), buf.as_ptr() as *const libc::c_void, buf.len(), flags, mem::transmute(ep.as_sockaddr()), ep.size() as libc::socklen_t) } {
        -1 => {
            let err = errno();
            if err == libc::EAGAIN || err == libc::EWOULDBLOCK {
                AsyncResult::WouldBlock
            } else {
                AsyncResult::Err(io::Error::from_raw_os_error(err))
            }
        },
        0 => AsyncResult::Err(io::Error::new(io::ErrorKind::WriteZero, "")),
        size => AsyncResult::Ok(size as usize),
    }
}

#[derive(Default, Clone)]
struct AtMark(pub i32);

impl<P: Protocol> IoControl<P> for AtMark {
    type Data = i32;

    fn name(&self) -> i32 {
        SIOCATMARK as i32
    }

    fn data(&mut self) -> &mut i32 {
        &mut self.0
    }
}

pub fn at_mark<S: AsRawFd, P: Protocol>(soc: &S) -> io::Result<bool> {
    let mut atmark: AtMark = AtMark::default();
    try!(ioctl::<S, P, AtMark>(soc, &mut atmark));
    Ok(atmark.0 != 0)
}

pub use libc::{EPOLLIN, EPOLLOUT, EPOLLERR, EPOLLHUP, EPOLLET, epoll_event};

extern {
    #[cfg_attr(target_os = "linux", link_name = "epoll_create1")]
    fn epoll_create1(flags: c_int) -> c_int;
}

pub fn epoll_create() -> io::Result<RawFd> {
    const EPOLL_CLOEXEC: c_int = libc::O_CLOEXEC;
    Ok(libc_try!(epoll_create1(EPOLL_CLOEXEC)))
}

#[allow(non_camel_case_types)]
pub enum EPOLL_CTL {
    EPOLL_CTL_ADD = libc::EPOLL_CTL_ADD as isize,
    EPOLL_CTL_DEL = libc::EPOLL_CTL_DEL as isize,
    #[allow(dead_code)]
    EPOLL_CTL_MOD = libc::EPOLL_CTL_MOD as isize,
}
pub use self::EPOLL_CTL::*;

pub fn epoll_ctl(epfd: RawFd, op: EPOLL_CTL, fd: RawFd, ev: &mut epoll_event) -> io::Result<()> {
    libc_try!(libc::epoll_ctl(epfd, op as i32, fd, ev));
    Ok(())
}

pub fn epoll_wait(epfd: RawFd, events: &mut [epoll_event], timeout: i32) -> usize {
    let ptr: *mut epoll_event = events.as_mut_ptr();
    let n = unsafe { libc::epoll_wait(epfd, ptr, events.len() as i32, timeout) };
    if n < 0 {
        0
    } else {
        n as usize
    }
}

#[test]
fn test_enum_epoll() {
    assert!(EPOLL_CTL_ADD as i32 == libc::EPOLL_CTL_ADD);
    assert!(EPOLL_CTL_DEL as i32 == libc::EPOLL_CTL_DEL);
    assert!(EPOLL_CTL_MOD as i32 == libc::EPOLL_CTL_MOD);
}

pub use libc::{addrinfo, freeaddrinfo};

fn str2c_char(src: &str, dst: &mut [c_char]) {
    let len = cmp::min(dst.len()-1, src.len());
    for (dst, src) in dst.iter_mut().zip(src.chars()) {
        *dst = src as c_char;
    };
    dst[len] = '\0' as c_char;
}

#[allow(unused_unsafe)]
pub unsafe fn getaddrinfo<P: Protocol>(pro: P, host: &str, port: &str, flags: i32) -> io::Result<*mut addrinfo> {
    let mut hints: libc::addrinfo = unsafe { mem::zeroed() };
    hints.ai_flags = flags;
    hints.ai_family = pro.family_type();
    hints.ai_socktype = pro.socket_type();
    hints.ai_protocol = pro.protocol_type();

    const ADDRINFO_NODE_MAX: usize = 256;
    let mut node: [c_char; ADDRINFO_NODE_MAX] = [0; ADDRINFO_NODE_MAX];
    let node = if !host.is_empty() {
        str2c_char(host, &mut node);
        hints.ai_flags |= AI_PASSIVE;
        node.as_ptr()
    } else {
        ptr::null()
    };

    const ADDRINFO_SERV_MAX: usize = 256;
    let mut serv: [c_char; ADDRINFO_SERV_MAX] = [0; ADDRINFO_SERV_MAX];
    let serv = if !port.is_empty() {
        str2c_char(port, &mut serv);
        serv.as_ptr()
    } else {
        ptr::null()
    };

    let mut base: *mut libc::addrinfo = ptr::null_mut();
    libc_try!(libc::getaddrinfo(node, serv, &hints, &mut base));
    Ok(base)
}

pub fn eventfd(initval: u32) -> io::Result<RawFd> {
    const EFD_CLOEXEC: i32 = libc::O_CLOEXEC;
    Ok(libc_try!(libc::eventfd(initval, EFD_CLOEXEC)))
}

#[repr(C)]
pub struct itimerspec {

    /// Interval for periodic timer
    pub it_interval: timespec,

    /// Initial expiration
    pub it_value: timespec,
}

extern {
    #[cfg_attr(target_os = "linux", link_name = "timerfd_create")]
    fn c_timerfd_create(clkid: c_int, flags: c_int) -> c_int;

    #[cfg_attr(target_os = "linux", link_name = "timerfd_settime")]
    fn c_timerfd_settime(fd: c_int, flags: c_int, new_value: *const itimerspec, old_value: *mut itimerspec) -> c_int;

    // #[cfg_attr(target_os = "linux", link_name = "timerfd_gettime")]
    // fn c_timerfd_gettime(fd: c_int, cur_value: *mut itimerspec) -> c_int;
}

pub fn timerfd_create(clkid: i32) -> io::Result<RawFd> {
    const TFD_CLOEXEC: i32 = libc::O_CLOEXEC;
    Ok(libc_try!(c_timerfd_create(clkid, TFD_CLOEXEC)))
}

#[allow(non_camel_case_types, dead_code)]
pub enum TFD_TIMER_TYPE {
    TFD_TIMER_RELTIME = 0,
    TFD_TIMER_ABSTIME = 1 << 0,
}
pub use self::TFD_TIMER_TYPE::*;

pub fn timerfd_settime<Fd: AsRawFd>(fd: &Fd, flags: TFD_TIMER_TYPE, new_value: &itimerspec) -> io::Result<()> {
    libc_try!(c_timerfd_settime(fd.as_raw_fd(), flags as i32, new_value, ptr::null_mut()));
    Ok(())
}

pub fn sleep_for(expiry: Expiry) -> io::Result<()> {
    let dur = expiry.wait_duration();
    let tv = timespec {
        tv_sec: dur.as_secs() as i64,
        tv_nsec: dur.subsec_nanos() as i64,
    };
    libc_try!(libc::nanosleep(&tv, ptr::null_mut()));
    Ok(())
}

#[repr(C)]
#[derive(Default, Clone)]
pub struct linger {
    pub l_onoff: libc::c_int,
    pub l_linger: libc::c_int,
}

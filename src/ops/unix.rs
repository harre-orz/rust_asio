use std::io;
use std::mem;
use std::cmp;
use std::ptr;
use std::time::{Duration};
use libc;
use socket::*;

pub use libc::{c_int, c_char};
pub use libc::{CLOCK_MONOTONIC, timeval, timespec};
pub use libc::{SHUT_RD, SHUT_WR, SHUT_RDWR};
pub use libc::{SOCK_DGRAM, SOCK_STREAM, SOCK_RAW, SOL_SOCKET, SO_REUSEADDR, SO_BROADCAST, SO_ACCEPTCONN, FIONREAD, IPPROTO_IP, IPPROTO_IPV6, IPV6_V6ONLY, SO_KEEPALIVE};
pub use libc::{AF_INET, AF_INET6, IPPROTO_TCP, sockaddr_in, sockaddr_in6, sockaddr_un, sockaddr_storage};

pub use std::os::unix::io::{RawFd, AsRawFd};
pub type RawSockAddrType = libc::sockaddr;
pub type RawSockLenType = libc::socklen_t;
pub const UNIX_PATH_MAX: usize = 108;
pub const SOCK_SEQPACKET: i32 = 5;
pub const AF_UNSPEC: i32 = 0;
pub const AF_LOCAL: i32 = 1;
pub const AI_PASSIVE: i32 = 0x0001;
pub const AI_NUMERICHOST: i32 = 0x0004;
pub const AI_NUMERICSERV: i32 = 0x0400;
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

pub fn operation_canceled() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Operation canceled")
}

pub trait AsRawSockAddr {
    fn as_raw_sockaddr(&self) -> &RawSockAddrType;
    fn as_mut_raw_sockaddr(&mut self) -> &mut RawSockAddrType;
    fn raw_socklen(&self) -> RawSockLenType;
}

pub fn socket<P: Protocol>(pro: P) -> io::Result<RawFd> {
    Ok(libc_try!(libc::socket(
        pro.family_type(),
        pro.socket_type() | libc::SOCK_CLOEXEC,
        pro.protocol_type()
    )))
}

pub fn close<Fd: AsRawFd>(fd: &Fd) -> io::Result<()> {
    libc_try!(libc::close(fd.as_raw_fd()));
    Ok(())
}

pub fn shutdown<Fd: AsRawFd>(fd: &Fd, how: Shutdown) -> io::Result<()> {
    libc_try!(libc::shutdown(fd.as_raw_fd(), how as i32));
    Ok(())
}

#[test]
fn test_enum_shutdown() {
    assert!(Shutdown::Read as i32 == SHUT_RD);
    assert!(Shutdown::Write as i32 == SHUT_WR);
    assert!(Shutdown::Both as i32 == SHUT_RDWR);
}

pub fn bind<Fd: AsRawFd, E: AsRawSockAddr>(fd: &Fd, ep: &E) -> io::Result<()> {
    libc_try!(libc::bind(fd.as_raw_fd(), ep.as_raw_sockaddr(), ep.raw_socklen()));
    Ok(())
}

pub fn connect<Fd: AsRawFd, E: AsRawSockAddr>(fd: &Fd, ep: &E) -> io::Result<()> {
    libc_try!(libc::connect(fd.as_raw_fd(), ep.as_raw_sockaddr(), ep.raw_socklen()));
    Ok(())
}

pub const SOMAXCONN: u32 = 126;
pub fn listen<Fd: AsRawFd>(fd: &Fd, backlog: u32) -> io::Result<()> {
    libc_try!(libc::listen(fd.as_raw_fd(), cmp::min(backlog, SOMAXCONN) as i32));
    Ok(())
}

pub fn accept<Fd: AsRawFd, E: AsRawSockAddr>(fd: &Fd, mut ep: E) -> io::Result<(RawFd, E)> {
    let mut socklen = ep.raw_socklen();
    let fd = libc_try!(libc::accept(fd.as_raw_fd(), ep.as_mut_raw_sockaddr(), &mut socklen));
    Ok((fd, ep))
}

pub fn getsockname<Fd: AsRawFd, E: AsRawSockAddr>(fd: &Fd, mut ep: E) -> io::Result<E> {
    let mut socklen = ep.raw_socklen();
    libc_try!(libc::getsockname(fd.as_raw_fd(), ep.as_mut_raw_sockaddr(), &mut socklen));
    Ok(ep)
}

pub fn getpeername<Fd: AsRawFd, E: AsRawSockAddr>(fd: &Fd, mut ep: E) -> io::Result<E> {
    let mut socklen = ep.raw_socklen();
    libc_try!(libc::getpeername(fd.as_raw_fd(), ep.as_mut_raw_sockaddr(), &mut socklen));
    Ok(ep)
}

pub fn recv<Fd: AsRawFd>(fd: &Fd, buf: &mut [u8], flags: i32) -> io::Result<usize> {
    let size = libc_try!(libc::recv(fd.as_raw_fd(), mem::transmute(buf.as_mut_ptr()), buf.len(), flags));
    Ok(size as usize)
}

pub fn recvfrom<Fd: AsRawFd, E: AsRawSockAddr>(fd: &Fd, buf: &mut [u8], flags: i32, mut ep: E) -> io::Result<(usize, E)> {
    let mut socklen = ep.raw_socklen();
    let size = libc_try!(libc::recvfrom(fd.as_raw_fd(), mem::transmute(buf.as_mut_ptr()), buf.len(), flags, ep.as_mut_raw_sockaddr(), &mut socklen));
    Ok((size as usize, ep))
}

pub fn send<Fd: AsRawFd>(fd: &Fd, buf: &[u8], flags: i32) -> io::Result<usize> {
    let size = libc_try!(libc::send(fd.as_raw_fd(), mem::transmute(buf.as_ptr()), buf.len(), flags));
    Ok(size as usize)

}

pub fn sendto<Fd: AsRawFd, E: AsRawSockAddr>(fd: &Fd, buf: &[u8], flags: i32, ep: &E) -> io::Result<usize> {
    let size = libc_try!(libc::sendto(fd.as_raw_fd(), mem::transmute(buf.as_ptr()), buf.len(), flags, ep.as_raw_sockaddr(), ep.raw_socklen()));
    Ok(size as usize)

}

pub fn ioctl<Fd: AsRawFd, S: Socket, T: IoControl<S>>(fd: &Fd, cmd: &mut T) -> io::Result<()> {
    libc_try!(libc::ioctl(fd.as_raw_fd(), cmd.name() as u64, cmd.data()));
    Ok(())
}

pub fn getsockopt<Fd: AsRawFd, S: Socket, T: GetSocketOption<S>>(fd: &Fd) -> io::Result<T> {
    let mut cmd = T::default();
    let mut datalen = 0;
    libc_try!(libc::getsockopt(fd.as_raw_fd(), cmd.level(), cmd.name(), mem::transmute(cmd.data_mut()), &mut datalen));
    cmd.resize(datalen as usize);
    Ok(cmd)
}

pub fn setsockopt<Fd: AsRawFd, S: Socket, T: SetSocketOption<S>>(fd: &Fd, cmd: &T) -> io::Result<()> {
    libc_try!(libc::setsockopt(fd.as_raw_fd(), cmd.level(), cmd.name(), mem::transmute(cmd.data()), cmd.size() as RawSockLenType));
    Ok(())
}

pub fn getflags<Fd: AsRawFd>(fd: &Fd) -> io::Result<i32> {
    Ok(libc_try!(libc::fcntl(fd.as_raw_fd(), libc::F_GETFL)))
}

pub fn setflags<Fd: AsRawFd>(fd: &Fd, flags: i32) -> io::Result<()> {
    libc_try!(libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags));
    Ok(())
}

pub fn getnonblock<Fd: AsRawFd>(fd: &Fd) -> io::Result<bool> {
    Ok((try!(getflags(fd)) & libc::O_NONBLOCK) != 0)
}

pub fn setnonblock<Fd: AsRawFd>(fd: &Fd, on: bool) -> io::Result<()> {
    let flags = try!(getflags(fd));
    setflags(fd, if on { flags | libc::O_NONBLOCK } else { flags & !libc::O_NONBLOCK })
}

pub use libc::{EPOLLIN, EPOLLOUT, EPOLLET, epoll_event};

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
    #[allow(dead_code)] EPOLL_CTL_MOD = libc::EPOLL_CTL_MOD as isize,
}
pub use self::EPOLL_CTL::*;

pub fn epoll_ctl(epfd: RawFd, op: EPOLL_CTL, fd: RawFd, ev: &mut epoll_event) -> io::Result<()> {
    libc_try!(libc::epoll_ctl(epfd, op as i32, fd, ev));
    Ok(())
}

pub fn epoll_wait(epfd: RawFd, events: &mut [epoll_event], timeout: &Duration) -> usize {
    let msec = (timeout.as_secs() * 1000 + timeout.subsec_nanos() as u64 / 1000000) as i32;
    let ptr: *mut epoll_event = events.as_mut_ptr();
    let n = unsafe { libc::epoll_wait(epfd, ptr, events.len() as i32, msec) };
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
        super::str2c_char(host, &mut node);
        hints.ai_flags |= AI_PASSIVE;
        node.as_ptr()
    } else {
        ptr::null()
    };

    const ADDRINFO_SERV_MAX: usize = 256;
    let mut serv: [c_char; ADDRINFO_SERV_MAX] = [0; ADDRINFO_SERV_MAX];
    let serv = if !port.is_empty() {
        super::str2c_char(port, &mut serv);
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

pub fn sleep_for<E>(res: Result<Duration, E>) -> io::Result<()> {
    match res {
        Ok(duration) => {
            let tv = timespec {
                tv_sec: duration.as_secs() as i64,
                tv_nsec: duration.subsec_nanos() as i64,
            };
            libc_try!(libc::nanosleep(&tv, ptr::null_mut()));
            Ok(())
        },
        Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Out of range")),
    }
}

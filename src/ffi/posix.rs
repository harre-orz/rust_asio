use ffi::Result;
use prelude::{
    Protocol,
    Endpoint,
    Socket,
    IoControl,
    GetSocketOption,
    SetSocketOption,
};

use std::mem;
use std::ptr;
use std::ffi::{CStr, CString};
use std::time::Duration;
use libc::{self, c_char, c_int, c_ulong};
use errno::{Errno, errno};

pub use std::os::unix::io::{AsRawFd, RawFd};
pub use libc::{
    AF_INET,
    AF_INET6,
    AF_UNIX,
    EAFNOSUPPORT,
    EAGAIN,
    ECANCELED,
    EINPROGRESS,
    EINTR,
    EINVAL,
    ETIMEDOUT,
    EWOULDBLOCK,
    F_GETFD,
    F_GETFL,
    F_SETFD,
    F_SETFL,
    IP_ADD_MEMBERSHIP,
    IP_DROP_MEMBERSHIP,
    IP_TTL,
    IP_MULTICAST_TTL,
    IP_MULTICAST_LOOP,
    IPPROTO_TCP,
    IPPROTO_IP,
    IPPROTO_IPV6,
    IPV6_V6ONLY,
    IPV6_MULTICAST_LOOP,
    SHUT_RD,
    SHUT_WR,
    SHUT_RDWR,
    FIONBIO,
    FD_SETSIZE,
    FD_CLOEXEC,
    O_NONBLOCK,
    SO_BROADCAST,
    SO_DEBUG,
    SO_DONTROUTE,
    SO_ERROR,
    SO_KEEPALIVE,
    SO_LINGER,
    SO_REUSEADDR,
    SO_RCVBUF,
    SO_RCVLOWAT,
    SO_SNDBUF,
    SO_SNDLOWAT,
    SOCK_DGRAM,
    SOCK_RAW,
    SOCK_SEQPACKET,
    SOCK_STREAM,
    SOL_SOCKET,
    TCP_NODELAY,
    addrinfo,
    in_addr,
    in6_addr,
    ip_mreq,
    ipv6_mreq,
    linger,
    sockaddr,
    sockaddr_in,
    sockaddr_in6,
    sockaddr_storage,
    sockaddr_un,
    socklen_t,
};

pub const IPV6_UNICAST_HOPS: c_int = 16;
pub const IPV6_MULTICAST_IF: c_int = 17;
pub const IPV6_MULTICAST_HOPS: c_int = 18;
pub const IP_MULTICAST_IF: c_int = 32;
pub const IPPROTO_ICMP: c_int = 1;
pub const IPPROTO_ICMPV6: c_int = 58;
pub const IPPROTO_UDP: c_int = 17;
pub const AF_UNSPEC: c_int = 0;
pub const AI_PASSIVE: c_int = 0x0001;
pub const AI_NUMERICHOST: c_int = 0x0004;
pub const AI_NUMERICSERV: c_int = 0x0400;
pub const SIOCATMARK: c_ulong = 0x8905;
pub use libc::FIONREAD;
#[cfg(target_os = "linux")] pub const IPV6_JOIN_GROUP: c_int = 20;
#[cfg(target_os = "linux")] pub const IPV6_LEAVE_GROUP: c_int = 21;
#[cfg(target_os = "linux")] pub use libc::{SOCK_CLOEXEC, SOCK_NONBLOCK};
#[cfg(target_os = "macos")] pub use libc::{IPV6_JOIN_GROUP, IPV6_LEAVE_GROUP};

#[cfg(not(target_os = "linux"))]
fn init_fd(fd: RawFd) -> Result<()> {
    let flags = unsafe { libc::fcntl(fd, F_GETFD) };
    if flags == -1 {
        return Err(errno());
    }
    unsafe { libc::fcntl(fd, F_SETFD, flags | FD_CLOEXEC); }
    let flags = unsafe { libc::fcntl(fd, F_GETFL) };
    if flags == -1 {
        return Err(errno());
    }
    unsafe { libc::fcntl(fd, F_SETFL, flags | O_NONBLOCK); }
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn accept<P, S>(soc: &S) -> Result<(RawFd, P::Endpoint)>
    where P: Protocol,
          S: Socket<P>,
{
    let mut ep = unsafe { soc.protocol().uninitialized() };
    let mut socklen = ep.capacity();
    match unsafe { libc::accept4(soc.as_raw_fd(), ep.as_mut_ptr(), &mut socklen, SOCK_CLOEXEC | SOCK_NONBLOCK) } {
        -1 => Err(errno()),
        fd => unsafe {
            ep.resize(socklen);
            Ok((fd, ep))
        },
    }
}

#[cfg(not(target_os = "linux"))]
pub fn accept<P, S>(soc: &S) -> Result<(RawFd, P::Endpoint)>
    where P: Protocol,
          S: Socket<P>,
{
    let mut ep = unsafe { soc.protocol().uninitialized() };
    let mut socklen = ep.capacity();
    match unsafe { libc::accept(soc.as_raw_fd(), ep.as_mut_ptr(), &mut socklen) } {
        -1 => Err(errno()),
        fd => unsafe {
            init_fd(fd)?;
            ep.resize(socklen);
            Ok((fd, ep))
        },
    }
}

pub fn bind<P, S>(soc: &S, ep: &P::Endpoint) -> Result<()>
    where P: Protocol,
          S: Socket<P>,
{
    if 0 != unsafe { libc::bind(soc.as_raw_fd(), ep.as_ptr(), ep.size()) } {
        return Err(errno())
    }
    Ok(())
}

pub fn listen<P, S>(soc: &S, backlog: i32) -> Result<()>
    where P: Protocol,
          S: Socket<P>,
{
    if 0 != unsafe { libc::listen(soc.as_raw_fd(), backlog) } {
        return Err(errno())
    }
    Ok(())
}

#[cfg(debug_assertions)]
pub fn close(fd: RawFd) {
    if 0 != unsafe { libc::close(fd) } {
        panic!("{}", errno());
    }
}

#[cfg(not(debug_assertions))]
pub fn close(fd: RawFd) {
    unsafe { libc::close(fd) };
}

pub fn connect<P, S>(soc: &S, ep: &P::Endpoint) -> Result<()>
    where P: Protocol,
          S: Socket<P>,
{
    if 0 != unsafe { libc::connect(soc.as_raw_fd(), ep.as_ptr(), ep.size()) } {
        return Err(errno())
    }
    Ok(())
}

pub fn freeaddrinfo(ai: *mut addrinfo) {
    unsafe { libc::freeaddrinfo(ai) }
}

pub fn getaddrinfo<P>(pro: &P, host: &str, port: &str, flags: i32) -> Result<*mut addrinfo>
    where P: Protocol
{
    // Fix: punycode
    let node = CString::new(host).unwrap();
    let serv = CString::new(port).unwrap();

    let mut hints: addrinfo = unsafe { mem::zeroed() };
    hints.ai_flags = flags;
    hints.ai_family = pro.family_type();
    hints.ai_socktype = pro.socket_type();
    hints.ai_protocol = pro.protocol_type();

    let mut base: *mut addrinfo = ptr::null_mut();
    if 0 != unsafe { libc::getaddrinfo(node.as_ptr(), serv.as_ptr(), &hints, &mut base) } {
        return Err(errno())
    }
    Ok(base)
}

pub fn gethostname() -> Result<String> {
    let mut name: [c_char; 65] = unsafe { mem::uninitialized() };
    if 0 == unsafe { libc::gethostname(name.as_mut_ptr(), mem::size_of_val(&name)) } {
        return Ok(String::from(unsafe { CStr::from_ptr(name.as_ptr()) }.to_str().unwrap()))
    }
    Err(errno())
}

pub fn getpeername<P, S>(soc: &S) -> Result<P::Endpoint>
    where P: Protocol,
          S: Socket<P>,
{
    let mut ep = unsafe { soc.protocol().uninitialized() };
    let mut socklen = ep.capacity();
    if 0 != unsafe { libc::getpeername(soc.as_raw_fd(), ep.as_mut_ptr(), &mut socklen) } {
        return Err(errno())
    }
    unsafe { ep.resize(socklen); }
    Ok(ep)
}

pub fn getsockname<P, S>(soc: &S) -> Result<P::Endpoint>
    where P: Protocol,
          S: Socket<P>,
{
    let mut ep = unsafe { soc.protocol().uninitialized() };
    let mut socklen = ep.capacity();
    if 0 != unsafe { libc::getsockname(soc.as_raw_fd(), ep.as_mut_ptr(), &mut socklen) } {
        return Err(errno())
    }
    unsafe { ep.resize(socklen); }
    Ok(ep)
}

pub fn getsockopt<P, S, C>(soc: &S) -> Result<C>
    where P: Protocol,
          S: Socket<P>,
          C: GetSocketOption<P>,
{
    let pro = soc.protocol();
    let mut cmd = C::default();
    let mut datalen = cmd.capacity();
    if 0 != unsafe {
        libc::getsockopt(soc.as_raw_fd(),
                         cmd.level(pro),
                         cmd.name(pro),
                         cmd.as_mut_ptr(),
                         &mut datalen) }
    {
        return Err(errno())
    }
    unsafe { cmd.resize(datalen); }
    Ok(cmd)
}

pub fn if_nametoindex(name: &str) -> Result<u32> {
    let name = CString::new(name).unwrap();
    match unsafe { libc::if_nametoindex(name.as_ptr()) } {
        0 => Err(errno()),
        i => Ok(i),
    }
}

pub fn ioctl<T, C>(fd: &T, cmd: &mut C) -> Result<()>
    where T: AsRawFd,
          C: IoControl,
{
    if -1 == unsafe { libc::ioctl(fd.as_raw_fd(), cmd.name(), cmd.as_mut_ptr()) } {
        return Err(errno())
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn pipe() -> Result<(RawFd, RawFd)> {
    let mut fd: [RawFd; 2] = unsafe { mem::uninitialized() };
    if 0 != unsafe { libc::pipe2(fd.as_mut_ptr(), O_CLOEXEC | O_NONBLOCK) } {
        return Err(errno())
    }
    Ok((fd[0], fd[1]))
}

#[cfg(not(target_os = "linux"))]
pub fn pipe() -> Result<(RawFd, RawFd)> {
    let mut fd: [RawFd; 2] = unsafe { mem::uninitialized() };
    if 0 != unsafe { libc::pipe(fd.as_mut_ptr()) } {
        return Err(errno())
    }
    init_fd(fd[0])?;
    init_fd(fd[1])?;
    Ok((fd[0], fd[1]))
}

pub fn read<T>(fd: &T, buf: &mut [u8]) -> Result<usize>
    where T: AsRawFd,
{
    match unsafe { libc::read(fd.as_raw_fd(), buf.as_mut_ptr() as *mut _, buf.len()) } {
        -1 => Err(errno()),
        len => Ok(len as usize)
    }
}

pub fn readable<T>(fd: &T, timeout: &Option<Duration>) -> Result<()>
    where T: AsRawFd,
{
    let mut fd = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let timeout = match timeout {
        &Some(dur) => dur.as_secs() as i32 * 1000 + dur.subsec_nanos() as i32 / 1000,
        &None => -1,
    };
    match unsafe { libc::poll(&mut fd, 1, timeout) } {
        1 => Ok(()),
        0 => Err(Errno(ETIMEDOUT)),
        _ => Err(errno()),
    }
}

pub fn recv<P, S>(soc: &S, buf:&mut [u8], flags: i32) -> Result<usize>
    where P: Protocol,
          S: Socket<P>,
{
    match unsafe { libc::recv(soc.as_raw_fd(), buf.as_mut_ptr() as *mut _, buf.len(), flags) } {
        -1 => Err(errno()),
        len => Ok(len as usize),
    }
}

pub fn recvfrom<P, S>(soc: &S, buf:&mut [u8], flags: i32) -> Result<(usize, P::Endpoint)>
    where P: Protocol,
          S: Socket<P>,
{
    let mut ep = unsafe { soc.protocol().uninitialized() };
    let mut salen = ep.capacity();
    match unsafe {
        libc::recvfrom(soc.as_raw_fd(),
                       buf.as_mut_ptr() as *mut _,
                       buf.len(),
                       flags,
                       ep.as_mut_ptr(),
                       &mut salen) }
    {
        -1 => Err(errno()),
        len => {
            unsafe { ep.resize(salen); }
            Ok((len as usize, ep))
        },
    }
}

pub fn send<P, S>(soc: &S, buf: &[u8], flags: i32) -> Result<usize>
    where P: Protocol,
          S: Socket<P>,
{
    match unsafe { libc::send(soc.as_raw_fd(),
                              buf.as_ptr() as *const _,
                              buf.len(),
                              flags) }
    {
        -1 => Err(errno()),
        len => Ok(len as usize),
    }
}

pub fn sendto<P, S>(soc: &S, buf: &[u8], flags: i32, ep: &P::Endpoint) -> Result<usize>
    where P: Protocol,
          S: Socket<P>,
{
    match unsafe {
        libc::sendto(soc.as_raw_fd(),
                     buf.as_ptr() as *const _,
                     buf.len(),
                     flags,
                     ep.as_ptr(),
                     ep.size()) }
    {
        -1 => Err(errno()),
        len => Ok(len as usize),
    }
}

pub fn setsockopt<P, S, C>(soc: &S, cmd: C) -> Result<()>
    where P: Protocol,
          S: Socket<P>,
          C: SetSocketOption<P>,
{
    let pro = soc.protocol();
    if 0 != unsafe {
        libc::setsockopt(soc.as_raw_fd(),
                         cmd.level(pro),
                         cmd.name(pro),
                         cmd.as_ptr(),
                         cmd.size()) }
    {
        return Err(errno())
    }
    Ok(())
}

pub fn shutdown<P, S, H>(soc: &S, how: H) -> Result<()>
    where P: Protocol,
          S: Socket<P>,
          H: Into<i32>,
{
    if 0 != unsafe { libc::shutdown(soc.as_raw_fd(), how.into()) } {
        return Err(errno())
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn socket<P>(pro: &P) -> Result<RawFd>
    where P: Protocol,
{
    match unsafe { libc::socket(pro.family_type(),
                                pro.socket_type(),
                                pro.protocol_type() | SOCK_NONBLOCK | SOCK_CLOEXEC) }
    {
        -1 => Err(errno()),
        fd => Ok(fd),
    }
}

#[cfg(not(target_os = "linux"))]
pub fn socket<P>(pro: &P) -> Result<RawFd>
    where P: Protocol,
{
    match unsafe { libc::socket(pro.family_type(),
                                pro.socket_type(),
                                pro.protocol_type()) }
    {
        -1 => Err(errno()),
        fd => {
            init_fd(fd)?;
            Ok(fd)
        },
    }
}

#[cfg(target_os = "linux")]
pub fn socketpair<P>(pro: &P) -> Result<(RawFd, RawFd)>
    where P: Protocol,
{
    let mut sv = [0; 2];
    if 0 != unsafe {
        libc::socketpair(pro.family_type(),
                         pro.socket_type(),
                         pro.protocol_type() | SOCK_NONBLOCK | SOCK_CLOEXEC,
                         sv.as_mut_ptr()) }
    {
        return Err(errno())
    }
    Ok((sv[0], sv[1]))
}

#[cfg(not(target_os = "linux"))]
pub fn socketpair<P>(pro: &P) -> Result<(RawFd, RawFd)>
    where P: Protocol,
{
    let mut sv = [0; 2];
    if 0 != unsafe {
        libc::socketpair(pro.family_type(),
                         pro.socket_type(),
                         pro.protocol_type(),
                         sv.as_mut_ptr()) }
    {
        return Err(errno())
    }
    init_fd(sv[0])?;
    init_fd(sv[1])?;
    Ok((sv[0], sv[1]))
}

pub fn write<T>(fd: &T, buf: &[u8]) -> Result<usize>
    where T: AsRawFd,
{
    match unsafe { libc::write(fd.as_raw_fd(), buf.as_ptr() as *mut _, buf.len()) } {
        -1 => Err(errno()),
        len => Ok(len as usize),
    }
}

pub fn writable<T>(fd: &T, timeout: &Option<Duration>) -> Result<()>
    where T: AsRawFd,
{
    let mut fd = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLOUT,
        revents: 0,
    };
    let timeout = match timeout {
        &Some(dur) => dur.as_secs() as i32 * 1000 + dur.subsec_nanos() as i32 / 1000,
        &None => -1,
    };
    match unsafe { libc::poll(&mut fd, 1, timeout) } {
        1 => Ok(()),
        0 => Err(Errno(ETIMEDOUT)),
        _ => Err(errno()),
    }
}

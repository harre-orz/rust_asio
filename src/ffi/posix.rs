use prelude::{Protocol, SockAddr, IoControl, GetSocketOption, SetSocketOption};

use std::io;
use std::mem;
use std::ptr;
use std::ffi::CStr;
use libc::{self, c_char, c_int, c_ulong, ssize_t};

pub use std::os::unix::io::{AsRawFd, RawFd};

pub use libc::{
    AF_INET,
    AF_INET6,
    AF_UNIX,
    EINTR,
    EINPROGRESS,
    ECANCELED,
    EAFNOSUPPORT,
    EAGAIN,
    EWOULDBLOCK,
    F_GETFL,
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
    SOCK_STREAM,
    SOCK_DGRAM,
    SOCK_RAW,
    SOCK_SEQPACKET,
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

pub const INVALID_SOCKET: c_int = -1;
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
// FIONREAD
#[cfg(target_os = "linux")] pub use libc::FIONREAD;
#[cfg(target_os = "macos")] pub const FIONREAD: c_ulong = 1074030207;
// IPV6_JOIN/LEAVE_GROUP
#[cfg(target_os = "linux")] pub const IPV6_JOIN_GROUP: c_int = 20;
#[cfg(target_os = "linux")] pub const IPV6_LEAVE_GROUP: c_int = 21;
#[cfg(target_os = "macos")] pub use libc::{IPV6_JOIN_GROUP, IPV6_LEAVE_GROUP};

pub unsafe fn accept<T, E>(t: &T, ep: &mut E, len: &mut socklen_t) -> RawFd
    where T: AsRawFd,
          E: SockAddr,
{
    libc::accept(
        t.as_raw_fd(),
        ep.as_mut() as *mut _ as *mut _,
        len
    )
}

pub fn bind<T, E>(t: &T, ep: &E) -> io::Result<()>
    where T: AsRawFd,
          E: SockAddr,
{
    libc_try!(libc::bind(
        t.as_raw_fd(),
        ep.as_ref() as *const _ as *const _,
        ep.size() as _
    ));
    Ok(())
}

pub fn cleanup() { }

pub fn close(fd: RawFd)
{
    libc_ign!(libc::close(fd));
}

pub unsafe fn connect<T, E>(t: &T, ep: &E) -> c_int
    where T: AsRawFd,
          E: SockAddr,
{
    libc::connect(
        t.as_raw_fd(),
        ep.as_ref() as *const _ as *const _,
        ep.size() as _
    )
}

pub fn freeaddrinfo(ai: *mut addrinfo) {
    unsafe { libc::freeaddrinfo(ai) }
}

pub fn getaddrinfo<P>(pro: &P, node: &CStr, serv: &CStr, flags: i32)
                      -> io::Result<*mut addrinfo>
    where P: Protocol,
{
    let mut hints: addrinfo = unsafe { mem::zeroed() };
    hints.ai_flags = flags;
    hints.ai_family = pro.family_type();
    hints.ai_socktype = pro.socket_type();
    hints.ai_protocol = pro.protocol_type();

    let node = if node.to_bytes().is_empty() {
        ptr::null()
    } else {
        node.as_ptr()
    };

    let serv = if serv.to_bytes().is_empty() {
        ptr::null()
    } else {
        serv.as_ptr()
    };

    let mut base: *mut addrinfo = ptr::null_mut();
    libc_try!(libc::getaddrinfo(node, serv, &hints, &mut base));
    Ok(base)
}

fn getflags<T>(t: &T) -> io::Result<i32>
    where T: AsRawFd,
{
    Ok(libc_try!(libc::fcntl(t.as_raw_fd(), F_GETFL)))
}

pub fn getnonblock<T>(t: &T) -> io::Result<bool>
    where T: AsRawFd,
{
    Ok((try!(getflags(t)) & libc::O_NONBLOCK) != 0)
}

pub fn gethostname() -> io::Result<String> {
    let mut name: [c_char; 65] = unsafe { mem::uninitialized() };
    libc_try!(libc::gethostname(name.as_mut_ptr(), mem::size_of_val(&name)));
    let cstr = unsafe { CStr::from_ptr(name.as_ptr()) };
    Ok(String::from(cstr.to_str().unwrap()))
}

pub fn getpeername<T, P>(t: &T, pro: &P) -> io::Result<P::Endpoint>
    where T: AsRawFd,
          P: Protocol,
{
    let mut ep = unsafe { pro.uninitialized() };
    let mut socklen = ep.capacity() as _;
    libc_try!(libc::getpeername(
        t.as_raw_fd(),
        ep.as_mut() as *mut _ as *mut _,
        &mut socklen
    ));
    unsafe { ep.resize(socklen as usize); }
    Ok(ep)
}

pub fn getsockname<T, P>(t: &T, pro: &P) -> io::Result<P::Endpoint>
    where T: AsRawFd,
          P: Protocol,
{
    let mut ep = unsafe { pro.uninitialized() };
    let mut socklen = ep.capacity() as _;
    libc_try!(libc::getsockname(
        t.as_raw_fd(),
        ep.as_mut() as *mut _ as *mut _,
        &mut socklen
    ));
    unsafe { ep.resize(socklen as usize); }
    Ok(ep)
}

pub fn getsockopt<T, P, C>(t: &T, pro: &P) -> io::Result<C>
    where T: AsRawFd,
          C: GetSocketOption<P>,
{
    let mut cmd = C::default();
    let mut datalen = cmd.capacity() as _;
    libc_try!(libc::getsockopt(
        t.as_raw_fd(),
        cmd.level(pro),
        cmd.name(pro),
        cmd.data_mut() as *mut _ as *mut _,
        &mut datalen
    ));
    cmd.resize(datalen as usize);
    Ok(cmd)
}

pub fn if_nametoindex(name: &CStr) -> io::Result<u32> {
    let ifi = unsafe { libc::if_nametoindex(name.as_ptr()) };
    if ifi == 0 {  // 0 が失敗
        Err(io::Error::last_os_error())
    } else {
        Ok(ifi)
    }
}

pub fn ioctl<T, C>(t: &T, cmd: &mut C) -> io::Result<()>
    where T: AsRawFd,
          C: IoControl,
{
    libc_try!(libc::ioctl(
        t.as_raw_fd(),
        cmd.name() as _,
        cmd.data()
    ));
    Ok(())
}

pub fn listen<T>(t: &T, backlog: i32) -> io::Result<()>
    where T: AsRawFd,
{
    libc_try!(libc::listen(t.as_raw_fd(), backlog));
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn pipe2(flags: i32) -> io::Result<(RawFd, RawFd)> {
    let mut fd: [RawFd; 2] = unsafe { mem::uninitialized() };
    libc_try!(libc::pipe2(fd.as_mut_ptr(), flags));
    Ok((fd[0], fd[1]))
}

pub fn pipe(flags: i32) -> io::Result<(RawFd, RawFd)> {
    let mut fd: [RawFd; 2] = unsafe { mem::uninitialized() };
    libc_try!(libc::pipe(fd.as_mut_ptr()));
    if flags != 0 {
        unsafe {
            if  libc::fcntl(fd[0], flags) != 0 ||
                libc::fcntl(fd[1], flags) != 0
            {
                let err = io::Error::last_os_error();
                libc::close(fd[0]);
                libc::close(fd[1]);
                return Err(err);
            }
        }
    }
    Ok((fd[0], fd[1]))
}

pub unsafe fn read<T>(t: &T, buf: &mut [u8]) -> ssize_t
    where T: AsRawFd,
{
    libc::read(
        t.as_raw_fd(),
        buf.as_mut_ptr() as *mut _,
        buf.len()
    )
}

pub unsafe fn recv<T>(t: &T, buf:&mut [u8], flags: i32) -> ssize_t
    where T: AsRawFd,
{
    libc::recv(
        t.as_raw_fd(),
        buf.as_mut_ptr() as *mut _,
        buf.len(),
        flags
    )
}

pub unsafe fn recvfrom<T, E>(t: &T, buf:&mut [u8], flags: i32,
                             ep: &mut E, len: &mut socklen_t) -> ssize_t
    where T: AsRawFd,
          E: SockAddr,
{
    libc::recvfrom(
        t.as_raw_fd(),
        buf.as_mut_ptr()as *mut _,
        buf.len(),
        flags,
        ep.as_mut() as *mut _ as *mut _,
        len,
    )
}

pub unsafe fn send<T>(t: &T, buf: &[u8], flags: i32) -> ssize_t
    where T: AsRawFd,
{
    libc::send(
        t.as_raw_fd(),
        buf.as_ptr() as *const _,
        buf.len(),
        flags
    )
}

pub unsafe fn sendto<T, E>(t: &T, buf: &[u8], flags: i32, ep: &E) -> ssize_t
    where T: AsRawFd,
          E: SockAddr,
{
    libc::sendto(
        t.as_raw_fd(),
        buf.as_ptr() as *const _,
        buf.len(),
        flags,
        ep.as_ref() as *const _ as *const _,
        ep.size() as _
    )
}

pub fn setflags<T>(t: &T, flags: i32) -> io::Result<()>
    where T: AsRawFd,
{
    libc_try!(libc::fcntl(t.as_raw_fd(), F_SETFL, flags));
    Ok(())
}

pub fn setnonblock<T>(t: &T, on: bool) -> io::Result<()>
    where T: AsRawFd,
{
    let flags = try!(getflags(t));
    setflags(t, if on { flags | O_NONBLOCK } else { flags & !O_NONBLOCK })
}

pub fn setsockopt<T, P, C>(t: &T, pro: &P, cmd: C) -> io::Result<()>
    where T: AsRawFd,
          C: SetSocketOption<P>,
{
    libc_try!(libc::setsockopt(
        t.as_raw_fd(),
        cmd.level(pro),
        cmd.name(pro),
        cmd.data() as *const _ as *const _,
        cmd.size() as _
    ));
    Ok(())
}

pub fn shutdown<T, H>(t: &T, how: H) -> io::Result<()>
    where T: AsRawFd,
          H: Into<i32>,
{
    libc_try!(libc::shutdown(t.as_raw_fd(), how.into()));
    Ok(())
}

pub fn socket<P>(pro: &P) -> io::Result<RawFd>
    where P: Protocol,
{
    Ok(libc_try!(libc::socket(
        pro.family_type(),
        pro.socket_type(),
        pro.protocol_type()
    )))
}

pub fn socketpair<P>(pro: &P) -> io::Result<(RawFd, RawFd)>
    where P: Protocol,
{
    let mut sv = [0; 2];
    libc_try!(libc::socketpair(
        pro.family_type(),
        pro.socket_type(),
        pro.protocol_type(),
        sv.as_mut_ptr()
    ));
    Ok((sv[0], sv[1]))
}

pub fn startup() { }

pub unsafe fn write<T>(t: &T, buf: &[u8]) -> ssize_t
    where T: AsRawFd,
{
    libc::write(
        t.as_raw_fd(),
        buf.as_ptr() as *mut _,
        buf.len()
    )
}

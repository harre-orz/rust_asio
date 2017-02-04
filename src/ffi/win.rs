use prelude::{Protocol, SockAddr, IoControl, GetSocketOption, SetSocketOption};

use std::io;
use std::mem;
use std::ptr;
use std::ffi::CStr;
use libc::{c_char, c_int, ssize_t};
use ws2_32;

pub use winapi::{
    ADDRINFOA as addrinfo,
    AF_INET,
    AF_INET6,
    AF_UNSPEC,
    WSAEINTR        as EINTR,
    WSAEINPROGRESS  as EINPROGRESS,
    WSA_E_CANCELLED as ECANCELED,
    WSAEWOULDBLOCK  as EWOULDBLOCK,
    ERROR_RETRY     as EAGAIN,
    WSAEAFNOSUPPORT as EAFNOSUPPORT,
    FIONBIO,
    INVALID_SOCKET,
    IP_TTL,
    IP_ADD_MEMBERSHIP,
    IP_DROP_MEMBERSHIP,
    IP_MULTICAST_IF,
    IP_MULTICAST_LOOP,
    IP_MULTICAST_TTL,
    IPPROTO,
    IPPROTO_ICMP,
    IPPROTO_ICMPV6,
    IPPROTO_IP,
    IPPROTO_IPV6,
    IPPROTO_UDP,
    IPPROTO_TCP,
    IPV6_V6ONLY,
    IPV6_MULTICAST_HOPS,
    IPV6_MULTICAST_IF,
    IPV6_MULTICAST_LOOP,
    IPV6_JOIN_GROUP,
    IPV6_LEAVE_GROUP,
    IPV6_UNICAST_HOPS,
    SO_DEBUG,
    SO_DONTROUTE,
    SO_KEEPALIVE,
    SO_ERROR,
    SO_LINGER,
    SO_RCVBUF,
    SO_RCVLOWAT,
    SO_SNDBUF,
    SO_SNDLOWAT,
    SO_REUSEADDR,
    SO_BROADCAST,
    SOCK_DGRAM,
    SOCK_RAW,
    SOCK_STREAM,
    SOCKADDR as sockaddr,
    SOCKADDR_IN as sockaddr_in,
    SOCKADDR_STORAGE as sockaddr_storage,
    SOCKET as RawFd,
    SOL_SOCKET,
    TCP_NODELAY,
    FD_SETSIZE,
    timeval,
    linger,
    in_addr,
    ip_mreq,
    in6_addr,
    ipv6_mreq,
    sockaddr_in6,
    socklen_t,
    fd_set,
};

pub use ws2_32::{select};

pub const FIONREAD: i32 =   0x4004667f;
pub const SIOCATMARK: i32 = 0x40047307;

pub const AI_PASSIVE: i32 = 1;
pub const AI_NUMERICHOST: i32 = 4;
pub const AI_NUMERICSERV: i32 = 8;

const SD_RECEIVE: i32 = 0x00;
const SD_SEND: i32 = 0x01;
const SD_BOTH: i32 = 0x02;

pub const SHUT_RD: i32 = SD_RECEIVE;
pub const SHUT_WR: i32 = SD_SEND;
pub const SHUT_RDWR: i32 = SD_BOTH;

pub trait AsRawFd {
    fn as_raw_fd(&self) -> RawFd;
}

impl super::IntoI32 for IPPROTO {
    fn i32(self) -> i32 {
        self.0 as i32
    }
}

pub unsafe fn accept<T, E>(t: &T, ep: &mut E, len: &mut socklen_t) -> RawFd
    where T: AsRawFd,
          E: SockAddr,
{
    ws2_32::accept(
        t.as_raw_fd(),
        ep.as_mut() as *mut _ as *mut _,
        len
    )
}

pub fn bind<T, E>(t: &T, ep: &E) -> io::Result<()>
    where T: AsRawFd,
          E: SockAddr,
{
    libc_try!(ws2_32::bind(
        t.as_raw_fd(),
        ep.as_ref() as *const _ as *const _,
        ep.size() as _
    ));
    Ok(())
}

pub fn cleanup() {
    libc_ign!(ws2_32::WSACleanup());
}

pub fn close(fd: RawFd) {
    libc_ign!(ws2_32::closesocket(fd));
}

pub unsafe fn connect<T, E>(t: &T, ep: &E) -> c_int
    where T: AsRawFd,
          E: SockAddr,
{
    ws2_32::connect(
        t.as_raw_fd(),
        ep.as_ref() as *const _ as *const _,
        ep.size() as _
    )
}

pub fn freeaddrinfo(ai: *mut addrinfo) {
    unsafe { ws2_32::freeaddrinfo(ai) }
}

pub fn getaddrinfo<P>(pro: &P, node: &CStr, serv: &CStr, flags: i32) -> io::Result<*mut addrinfo>
    where P: Protocol
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
    libc_try!(ws2_32::getaddrinfo(node, serv, &hints, &mut base));
    Ok(base)
}

pub fn gethostname() -> io::Result<String> {
    let mut name: [c_char; 65] = unsafe { mem::uninitialized() };
    libc_try!(ws2_32::gethostname(name.as_mut_ptr(), mem::size_of_val(&name) as _));
    let cstr = unsafe { CStr::from_ptr(name.as_ptr()) };
    Ok(String::from(cstr.to_str().unwrap()))
}

pub fn getpeername<T, P>(t: &T, pro: &P) -> io::Result<P::Endpoint>
    where T: AsRawFd,
          P: Protocol,
{
    let mut ep = unsafe { pro.uninitialized() };
    let mut socklen = ep.capacity() as _;
    libc_try!(ws2_32::getpeername(
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
    libc_try!(ws2_32::getsockname(
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
    libc_try!(ws2_32::getsockopt(
        t.as_raw_fd(),
        cmd.level(pro),
        cmd.name(pro),
        cmd.data_mut() as *mut _ as *mut _,
        &mut datalen
    ));
    cmd.resize(datalen as usize);
    Ok(cmd)
}

pub fn if_nametoindex(_: &CStr) -> Result<u32, ()> {
    Err(())
}

pub fn ioctl<T, C>(t: &T, cmd: &mut C) -> io::Result<()>
    where T: AsRawFd,
          C: IoControl,
{
    libc_try!(ws2_32::ioctlsocket(
        t.as_raw_fd(),
        cmd.name(),
        cmd.data() as *mut _ as *mut _
    ));
    Ok(())
}

pub fn listen<T>(t: &T, backlog: i32) -> io::Result<()>
    where T: AsRawFd,
{
    libc_try!(ws2_32::listen(t.as_raw_fd(), backlog));
    Ok(())
}

pub unsafe fn read<T>(t: &T, buf: &mut [u8]) -> ssize_t
    where T: AsRawFd,
{
    recv(t, buf, 0)
}

pub unsafe fn recv<T>(t: &T, buf:&mut [u8], flags: i32) -> ssize_t
    where T: AsRawFd,
{
    ws2_32::recv(
        t.as_raw_fd(),
        buf.as_mut_ptr() as *mut _,
        buf.len() as c_int,
        flags
    ) as isize
}

pub unsafe fn recvfrom<T, E>(t: &T, buf:&mut [u8], flags: i32, ep: &mut E, len: &mut socklen_t) -> ssize_t
    where T: AsRawFd,
          E: SockAddr,
{
    ws2_32::recvfrom(
        t.as_raw_fd(),
        buf.as_mut_ptr() as *mut _,
        buf.len() as c_int,
        flags,
        ep.as_mut() as *mut _ as *mut _,
        len,
    ) as ssize_t
}

pub unsafe fn send<T>(t: &T, buf: &[u8], flags: i32) -> ssize_t
    where T: AsRawFd,
{
    ws2_32::send(
        t.as_raw_fd(),
        buf.as_ptr() as *const _,
        buf.len() as _,
        flags
    ) as ssize_t
}

pub unsafe fn sendto<T, E>(t: &T, buf: &[u8], flags: i32, ep: &E) -> ssize_t
    where T: AsRawFd,
          E: SockAddr,
{
   ws2_32::sendto(
       t.as_raw_fd(),
       buf.as_ptr() as *const _,
       buf.len() as _,
       flags,
       ep.as_ref() as *const _ as *const _,
       ep.size() as _
    ) as ssize_t
}

pub fn setsockopt<T, P, C>(t: &T, pro: &P, cmd: C) -> io::Result<()>
    where T: AsRawFd,
          C: SetSocketOption<P>,
{
    libc_try!(ws2_32::setsockopt(
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
    libc_try!(ws2_32::shutdown(t.as_raw_fd(), how.into()));
    Ok(())
}

pub fn socket<P>(pro: &P) -> io::Result<RawFd>
    where P: Protocol,
{
    let s = unsafe { ws2_32::socket(
        pro.family_type(),
        pro.socket_type(),
        pro.protocol_type()
    ) };
    if s != INVALID_SOCKET {
        Ok(s)
    } else  {
        Err(io::Error::last_os_error())
    }
}

pub fn startup() {
    use winapi::WSADATA;

    let mut wsa: WSADATA = unsafe { mem::uninitialized() };
    libc_ign!(ws2_32::WSAStartup(2, &mut wsa));
}

pub unsafe fn write<T>(t: &T, buf: &[u8]) -> ssize_t
    where T: AsRawFd,
{
    send(t, buf, 0)
}

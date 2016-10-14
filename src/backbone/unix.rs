use std::io;
use std::cmp;
use std::ptr;
use std::mem;
use std::time;
use std::ffi::{CStr, CString};
use libc;
use {Shutdown, Protocol, SockAddr, IoControl, GetSocketOption, SetSocketOption};
use super::{RawFd, AsRawFd};

// time
pub use libc::{timespec};

// netinet/in
pub use libc::{sockaddr, sockaddr_in, sockaddr_in6, sockaddr_un, sockaddr_storage,
               in_addr, in6_addr,
               IPPROTO_IP, IPPROTO_IPV6, IPPROTO_TCP};
pub const IPPROTO_ICMP: i32 = 1;
pub const IPPROTO_ICMPV6: i32 = 58;

// stropts
pub use libc::{FIONREAD, TIOCOUTQ};
pub const SIOCATMARK: i32  = 0x8905;

// sys/socket // AF_*, PF_*, IP*, IPV6_*, SOL_*, SO_*
pub use libc::{SHUT_RD, SHUT_WR, SHUT_RDWR,
               AF_INET, AF_INET6, AF_UNIX as AF_LOCAL,
               SOCK_STREAM, SOCK_DGRAM, SOCK_RAW, SOCK_SEQPACKET,
               SOL_SOCKET,
               SO_BROADCAST, SO_DEBUG, SO_DONTROUTE, SO_KEEPALIVE, SO_LINGER, SO_REUSEADDR, SO_RCVBUF, SO_RCVLOWAT,
               IPV6_V6ONLY, TCP_NODELAY,
               IP_TTL, IP_MULTICAST_TTL,
               IP_MULTICAST_LOOP, IPV6_MULTICAST_LOOP,
               IP_ADD_MEMBERSHIP, IP_DROP_MEMBERSHIP, ip_mreq, ipv6_mreq};
pub const AF_UNSPEC: i32 = 0;
pub const IPV6_UNICAST_HOPS: i32 = 16;
pub const IPV6_MULTICAST_HOPS: i32 = 18;
pub const IPV6_JOIN_GROUP: i32 = 20;
pub const IPV6_LEAVE_GROUP: i32 = 21;
pub const IP_MULTICAST_IF: i32 = 32;
pub const IPV6_MULTICAST_IF: i32 = 17;

// netdb
pub use libc::{addrinfo, freeaddrinfo};
pub const AI_PASSIVE: i32 = 0x0001;
//pub const AI_NUMERICHOST: i32 = 0x0004;
pub const AI_NUMERICSERV: i32 = 0x0400;

pub fn close<T: AsRawFd>(fd: &T) {
    let _err = unsafe { libc::close(fd.as_raw_fd()) };
    debug_assert_eq!(_err, 0);
}

pub fn ioctl<T: AsRawFd, C: IoControl>(fd: &T, cmd: &mut C) -> io::Result<()> {
    libc_try!(libc::ioctl(fd.as_raw_fd(), cmd.name() as u64, cmd.data()));
    Ok(())
}

pub fn getflags<T: AsRawFd>(fd: &T) -> io::Result<i32> {
    Ok(libc_try!(libc::fcntl(fd.as_raw_fd(), libc::F_GETFL)))
}

pub fn setflags<T: AsRawFd>(fd: &T, flags: i32) -> io::Result<()> {
    libc_try!(libc::fcntl(fd.as_raw_fd(), libc::F_SETFL, flags));
    Ok(())
}

pub fn getnonblock<T: AsRawFd>(fd: &T) -> io::Result<bool> {
    Ok((try!(getflags(fd)) & libc::O_NONBLOCK) != 0)
}

pub fn setnonblock<T: AsRawFd>(fd: &T, on: bool) -> io::Result<()> {
    let flags = try!(getflags(fd));
    setflags(fd, if on { flags | libc::O_NONBLOCK } else { flags & !libc::O_NONBLOCK })
}

pub fn shutdown<T: AsRawFd>(fd: &T, how: Shutdown) -> io::Result<()> {
    libc_try!(libc::shutdown(fd.as_raw_fd(), how as i32));
    Ok(())
}

pub fn socket<P: Protocol>(pro: &P) -> io::Result<RawFd> {
    Ok(libc_try!(libc::socket(pro.family_type() as i32, pro.socket_type(), pro.protocol_type())))
}

pub fn bind<T: AsRawFd, E: SockAddr>(fd: &T, ep: &E) -> io::Result<()> {
    libc_try!(libc::bind(fd.as_raw_fd(), ep.as_sockaddr() as *const _ as *const libc::sockaddr, ep.size() as libc::socklen_t));
    Ok(())
}

pub const SOMAXCONN: u32 = 126;
pub fn listen<T: AsRawFd>(fd: &T, backlog: u32) -> io::Result<()> {
    libc_try!(libc::listen(fd.as_raw_fd(), cmp::min(backlog, SOMAXCONN) as i32));
    Ok(())
}

pub fn getsockname<T: AsRawFd, E: SockAddr>(fd: &T, mut ep: E) -> io::Result<E> {
    let mut socklen = ep.capacity() as libc::socklen_t;
    libc_try!(libc::getsockname(fd.as_raw_fd(), ep.as_mut_sockaddr() as *mut _ as *mut libc::sockaddr, &mut socklen));
    unsafe { ep.resize(socklen as usize); }
    Ok(ep)
}

pub fn getpeername<T: AsRawFd, E: SockAddr>(fd: &T, mut ep: E) -> io::Result<E> {
    let mut socklen = ep.capacity() as libc::socklen_t;
    libc_try!(libc::getpeername(fd.as_raw_fd(), ep.as_mut_sockaddr() as *mut _ as *mut libc::sockaddr, &mut socklen));
    unsafe { ep.resize(socklen as usize); }
    Ok(ep)
}

pub fn getsockopt<T: AsRawFd, P: Protocol, C: GetSocketOption<P>>(fd: &T, pro: &P) -> io::Result<C> {
    let mut cmd = C::default();
    let mut datalen = 0;
    libc_try!(libc::getsockopt(fd.as_raw_fd(), cmd.level(pro), cmd.name(pro), cmd.data_mut() as *mut _ as *mut libc::c_void, &mut datalen));
    cmd.resize(datalen as usize);
    Ok(cmd)
}

pub fn setsockopt<T: AsRawFd, P: Protocol, C: SetSocketOption<P>>(fd: &T, pro: &P, cmd: C) -> io::Result<()> {
    libc_try!(libc::setsockopt(fd.as_raw_fd(), cmd.level(pro), cmd.name(pro), cmd.data() as *const  _ as *const libc::c_void, cmd.size() as libc::socklen_t));
    Ok(())
}

pub fn sleep_for(time: time::Duration) -> io::Result<()> {
    let tv = timespec {
        tv_sec: time.as_secs() as i64,
        tv_nsec: time.subsec_nanos() as i64,
    };
    libc_try!(libc::nanosleep(&tv, ptr::null_mut()));
    Ok(())
}

pub struct AddrInfo(*mut libc::addrinfo);

impl AddrInfo {
    pub fn as_ptr(&self) -> *mut libc::addrinfo {
        self.0
    }
}

impl Drop for AddrInfo {
    fn drop(&mut self) {
        unsafe { libc::freeaddrinfo(self.0) }
    }
}

pub fn getaddrinfo<P: Protocol, T: Into<Vec<u8>>, U: Into<Vec<u8>>>(pro: P, host: T, port: U, flags: i32) -> io::Result<AddrInfo> {
    let mut hints: libc::addrinfo = unsafe { mem::zeroed() };
    hints.ai_flags = flags;
    hints.ai_family = pro.family_type();
    hints.ai_socktype = pro.socket_type();
    hints.ai_protocol = pro.protocol_type();

    let host = CString::new(host);
    let node = match &host {
        &Ok(ref node) if node.as_bytes().len() > 0
            => node.as_ptr(),
        _
            => ptr::null(),
    };

    let port = CString::new(port);
    let serv = match &port {
        &Ok(ref serv) if serv.as_bytes().len() > 0
            => serv.as_ptr(),
        _
            => ptr::null(),
    };

    let mut base: *mut libc::addrinfo = ptr::null_mut();
    libc_try!(libc::getaddrinfo(node, serv, &hints, &mut base));
    Ok(AddrInfo(base))
}

pub fn socketpair<P: Protocol>(pro: &P) -> io::Result<(RawFd, RawFd)> {
    let mut sv = [0; 2];
    libc_try!(libc::socketpair(pro.family_type(), pro.socket_type(), pro.protocol_type(), sv.as_mut_ptr()));
    Ok((sv[0], sv[1]))
}

pub fn gethostname() -> io::Result<String> {
    let mut name: [libc::c_char; 65] = unsafe { mem::uninitialized() };
    libc_try!(libc::gethostname(name.as_mut_ptr(), mem::size_of_val(&name)));
    let cstr = unsafe { CStr::from_ptr(name.as_ptr()) };
    Ok(String::from(cstr.to_str().unwrap()))
}

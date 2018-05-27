//use core::{Protocol, Endpoint, Socket, IoControl, GetSocketOption, SetSocketOption};

use std::io;
use std::mem;
use std::ptr;
use std::fmt;
use std::ffi::CStr;
use libc::{c_char, c_int, ssize_t};

pub use winapi::shared::inaddr::in_addr;
pub use winapi::shared::in6addr::in6_addr;
pub use winapi::shared::ws2def::{ADDRINFOA as addrinfo, AF_INET6, IPPROTO_ICMPV6, IPPROTO_IPV6, SOCKADDR as sockaddr,
                                 SOCKADDR_IN as sockaddr_in, SOCKADDR_STORAGE as sockaddr_storage, AF_INET, AF_UNSPEC,
                                 IPPROTO, IPPROTO_ICMP, IPPROTO_IP, IPPROTO_TCP, IPPROTO_UDP, SOCK_DGRAM, SOCK_RAW,
                                 SOCK_STREAM, SOL_SOCKET, SO_BROADCAST, SO_DEBUG, SO_DONTROUTE, SO_ERROR, SO_KEEPALIVE,
                                 SO_LINGER, SO_RCVBUF, SO_RCVLOWAT, SO_REUSEADDR, SO_SNDBUF, SO_SNDLOWAT, TCP_NODELAY,
			         AI_PASSIVE, AI_NUMERICHOST, AI_NUMERICSERV};
pub use winapi::shared::ws2ipdef::{IP_MREQ as ip_mreq, IPV6_JOIN_GROUP, IPV6_LEAVE_GROUP, IPV6_MULTICAST_HOPS,
                                   IPV6_MULTICAST_IF, IPV6_MULTICAST_LOOP, IPV6_UNICAST_HOPS, IPV6_V6ONLY, 
                                   IP_ADD_MEMBERSHIP, IP_DROP_MEMBERSHIP, IP_MULTICAST_IF,
			           IP_MULTICAST_LOOP, IP_MULTICAST_TTL, IP_TTL, IPV6_MREQ as ipv6_mreq,
			           SOCKADDR_IN6_LH as sockaddr_in6};
pub use winapi::um::winsock2::{self, fd_set, linger, timeval, SOCKET as RawFd, FD_SETSIZE, FIONBIO, INVALID_SOCKET,
                               select, WSAGetLastError, FIONREAD, SIOCATMARK, SD_RECEIVE, SD_SEND, SD_BOTH};
pub use winapi::um::ws2tcpip::socklen_t;

pub const SHUT_RD: i32 = SD_RECEIVE;
pub const SHUT_WR: i32 = SD_SEND;
pub const SHUT_RDWR: i32 = SD_BOTH;

pub trait AsRawFd {
    fn as_raw_fd(&self) -> RawFd;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct SystemError(c_int);

impl SystemError {
    pub fn last_error() -> Self {
        SystemError(unsafe { WSAGetLastError() })
    }
}

impl Default for SystemError {
    fn default() -> Self {
        SystemError(0)
    }
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use winapi::um::winbase::{FormatMessageW, FORMAT_MESSAGE_IGNORE_INSERTS};
    	use winapi::um::winnt::WCHAR;
        use winapi::shared::minwindef::DWORD;

        let mut buf = [0 as WCHAR; 2048];
	match unsafe { FormatMessageW(
            FORMAT_MESSAGE_IGNORE_INSERTS,
	    ptr::null_mut(),
	    self.0 as DWORD,
	    0x0800,
	    buf.as_mut_ptr(),
	    buf.len() as DWORD,
	    ptr::null_mut()) }
	{
	    0 => write!(f, "invalid error code: {}", self.0),
	    len => write!(f, "{}", String::from_utf16_lossy(&buf[..len as usize])),
	}
    }
}

impl From<SystemError> for io::Error {
    fn from(err: SystemError) -> Self {
        io::Error::from_raw_os_error(err.0)
    }
}

/// Permission denied.
pub const ACCESS_DENIED: SystemError = SystemError(winsock2::WSAEACCES);

/// Address family not supported by protocol.
pub const ADDRESS_FAMILY_NOT_SUPPORTED: SystemError = SystemError(winsock2::WSAEAFNOSUPPORT);

/// Address already in use.
pub const ADDRESS_IN_USE: SystemError = SystemError(winsock2::WSAEADDRINUSE);

/// Transport endpoint is already connected.
pub const ALREADY_CONNECTED: SystemError = SystemError(winsock2::WSAEISCONN);

/// Operation already in progress.
pub const ALREADY_STARTED: SystemError = SystemError(winsock2::WSAEALREADY);

/// A connection has been aborted.
pub const CONNECTION_ABORTED: SystemError = SystemError(winsock2::WSAECONNABORTED);

/// connection refused.
pub const CONNECTION_REFUSED: SystemError = SystemError(winsock2::WSAECONNREFUSED);

/// Connection reset by peer.
pub const CONNECTION_RESET: SystemError = SystemError(winsock2::WSAECONNRESET);

/// Bad file descriptor.
pub const BAD_DESCRIPTOR: SystemError = SystemError(winsock2::WSAEBADF);

/// Bad address.
pub const FAULT: SystemError = SystemError(winsock2::WSAEFAULT);

/// No route to host.
pub const HOST_UNREACHABLE: SystemError = SystemError(winsock2::WSAEHOSTUNREACH);

/// peration now in progress.
pub const IN_PROGRESS: SystemError = SystemError(winsock2::WSAEINPROGRESS);

/// Interrupted system call.
pub const INTERRUPTED: SystemError = SystemError(winsock2::WSAEINTR);

/// Invalid argument.
pub const INVALID_ARGUMENT: SystemError = SystemError(winsock2::WSAEINVAL);

/// Message to long.
pub const MESSAGE_SIZE: SystemError = SystemError(winsock2::WSAEMSGSIZE);

/// The name was too long.
pub const NAME_TOO_LONG: SystemError = SystemError(winsock2::WSAENAMETOOLONG);

/// Network is down.
pub const NETWORK_DOWN: SystemError = SystemError(winsock2::WSAENETDOWN);

/// Network dropped connection on reset.
pub const NETWORK_RESET: SystemError = SystemError(winsock2::WSAENETRESET);

/// Network is unreachable.
pub const NETWORK_UNREACHABLE: SystemError = SystemError(winsock2::WSAENETUNREACH);

/// Too many open files.
pub const NO_DESCRIPTORS: SystemError = SystemError(winsock2::WSAEMFILE);

/// No buffer space available.
pub const NO_BUFFER_SPACE: SystemError = SystemError(winsock2::WSAENOBUFS);

/// Protocol not available.
pub const NO_PROTOCOL_OPTION: SystemError = SystemError(winsock2::WSAENOPROTOOPT);

/// Transport endpoint is not connected.
pub const NOT_CONNECTED: SystemError = SystemError(winsock2::WSAENOTCONN);

/// Socket operation on non-socket.
pub const NOT_SOCKET: SystemError = SystemError(winsock2::WSAENOTSOCK);

/// Operation cancelled.
pub const OPERATION_CANCELED: SystemError = SystemError(winsock2::WSAECANCELLED);

/// Operation not supported.
pub const OPERATION_NOT_SUPPORTED: SystemError = SystemError(winsock2::WSAEOPNOTSUPP);

/// Cannot send after transport endpoint shutdown.
pub const SHUT_DOWN: SystemError = SystemError(winsock2::WSAESHUTDOWN);

/// Connection timed out.
pub const TIMED_OUT: SystemError = SystemError(winsock2::WSAETIMEDOUT);

/// Resource temporarily unavailable.
pub const TRY_AGAIN: SystemError = SystemError(winsock2::TRY_AGAIN);

/// The socket is marked non-blocking and the requested operation would block.
pub const WOULD_BLOCK: SystemError = SystemError(winsock2::WSAEWOULDBLOCK);

// pub unsafe fn accept<T, E>(t: &T, ep: &mut E, len: &mut socklen_t) -> RawFd
// where
//     T: AsRawFd,
//     E: SockAddr,
// {
//     winsock2::accept(t.as_raw_fd(), ep.as_mut() as *mut _ as *mut _, len)
// }

// pub fn bind<T, E>(t: &T, ep: &E) -> io::Result<()>
// where
//     T: AsRawFd,
//     E: SockAddr,
// {
//     libc_try!(ws2_32::bind(
//         t.as_raw_fd(),
//         ep.as_ref() as *const _ as *const _,
//         ep.size() as _,
//     ));
//     Ok(())
// }

// pub fn cleanup() {
//     libc_ign!(ws2_32::WSACleanup());
// }

// pub fn close(fd: RawFd) {
//     libc_ign!(ws2_32::closesocket(fd));
// }

// pub unsafe fn connect<T, E>(t: &T, ep: &E) -> c_int
// where
//     T: AsRawFd,
//     E: SockAddr,
// {
//     ws2_32::connect(
//         t.as_raw_fd(),
//         ep.as_ref() as *const _ as *const _,
//         ep.size() as _,
//     )
// }

// pub fn freeaddrinfo(ai: *mut addrinfo) {
//     unsafe { ws2_32::freeaddrinfo(ai) }
// }

// pub fn getaddrinfo<P>(pro: &P, node: &CStr, serv: &CStr, flags: i32) -> io::Result<*mut addrinfo>
// where
//     P: Protocol,
// {
//     let mut hints: addrinfo = unsafe { mem::zeroed() };
//     hints.ai_flags = flags;
//     hints.ai_family = pro.family_type();
//     hints.ai_socktype = pro.socket_type();
//     hints.ai_protocol = pro.protocol_type();

//     let node = if node.to_bytes().is_empty() {
//         ptr::null()
//     } else {
//         node.as_ptr()
//     };

//     let serv = if serv.to_bytes().is_empty() {
//         ptr::null()
//     } else {
//         serv.as_ptr()
//     };

//     let mut base: *mut addrinfo = ptr::null_mut();
//     libc_try!(ws2_32::getaddrinfo(node, serv, &hints, &mut base));
//     Ok(base)
// }

// pub fn gethostname() -> io::Result<String> {
//     let mut name: [c_char; 65] = unsafe { mem::uninitialized() };
//     libc_try!(ws2_32::gethostname(
//         name.as_mut_ptr(),
//         mem::size_of_val(&name) as _,
//     ));
//     let cstr = unsafe { CStr::from_ptr(name.as_ptr()) };
//     Ok(String::from(cstr.to_str().unwrap()))
// }

// pub fn getpeername<T, P>(t: &T, pro: &P) -> io::Result<P::Endpoint>
// where
//     T: AsRawFd,
//     P: Protocol,
// {
//     let mut ep = unsafe { pro.uninitialized() };
//     let mut socklen = ep.capacity() as _;
//     libc_try!(ws2_32::getpeername(
//         t.as_raw_fd(),
//         ep.as_mut() as *mut _ as *mut _,
//         &mut socklen,
//     ));
//     unsafe {
//         ep.resize(socklen as usize);
//     }
//     Ok(ep)
// }

// pub fn getsockname<T, P>(t: &T, pro: &P) -> io::Result<P::Endpoint>
// where
//     T: AsRawFd,
//     P: Protocol,
// {
//     let mut ep = unsafe { pro.uninitialized() };
//     let mut socklen = ep.capacity() as _;
//     libc_try!(ws2_32::getsockname(
//         t.as_raw_fd(),
//         ep.as_mut() as *mut _ as *mut _,
//         &mut socklen,
//     ));
//     unsafe {
//         ep.resize(socklen as usize);
//     }
//     Ok(ep)
// }

// pub fn getsockopt<T, P, C>(t: &T, pro: &P) -> io::Result<C>
// where
//     T: AsRawFd,
//     C: GetSocketOption<P>,
// {
//     let mut cmd = C::default();
//     let mut datalen = cmd.capacity() as _;
//     libc_try!(ws2_32::getsockopt(
//         t.as_raw_fd(),
//         cmd.level(pro),
//         cmd.name(pro),
//         cmd.data_mut() as *mut _ as *mut _,
//         &mut datalen,
//     ));
//     cmd.resize(datalen as usize);
//     Ok(cmd)
// }

// pub fn if_nametoindex(_: &CStr) -> Result<u32, ()> {
//     Err(())
// }

// pub fn ioctl<T, C>(t: &T, cmd: &mut C) -> io::Result<()>
// where
//     T: AsRawFd,
//     C: IoControl,
// {
//     libc_try!(ws2_32::ioctlsocket(
//         t.as_raw_fd(),
//         cmd.name(),
//         cmd.data() as *mut _ as *mut _,
//     ));
//     Ok(())
// }

// pub fn listen<T>(t: &T, backlog: i32) -> io::Result<()>
// where
//     T: AsRawFd,
// {
//     libc_try!(ws2_32::listen(t.as_raw_fd(), backlog));
//     Ok(())
// }

// pub unsafe fn read<T>(t: &T, buf: &mut [u8]) -> ssize_t
// where
//     T: AsRawFd,
// {
//     recv(t, buf, 0)
// }

// pub unsafe fn recv<T>(t: &T, buf: &mut [u8], flags: i32) -> ssize_t
// where
//     T: AsRawFd,
// {
//     ws2_32::recv(
//         t.as_raw_fd(),
//         buf.as_mut_ptr() as *mut _,
//         buf.len() as c_int,
//         flags,
//     ) as isize
// }

// pub unsafe fn recvfrom<T, E>(
//     t: &T,
//     buf: &mut [u8],
//     flags: i32,
//     ep: &mut E,
//     len: &mut socklen_t,
// ) -> ssize_t
// where
//     T: AsRawFd,
//     E: SockAddr,
// {
//     ws2_32::recvfrom(
//         t.as_raw_fd(),
//         buf.as_mut_ptr() as *mut _,
//         buf.len() as c_int,
//         flags,
//         ep.as_mut() as *mut _ as *mut _,
//         len,
//     ) as ssize_t
// }

// pub unsafe fn send<T>(t: &T, buf: &[u8], flags: i32) -> ssize_t
// where
//     T: AsRawFd,
// {
//     ws2_32::send(
//         t.as_raw_fd(),
//         buf.as_ptr() as *const _,
//         buf.len() as _,
//         flags,
//     ) as ssize_t
// }

// pub unsafe fn sendto<T, E>(t: &T, buf: &[u8], flags: i32, ep: &E) -> ssize_t
// where
//     T: AsRawFd,
//     E: SockAddr,
// {
//     ws2_32::sendto(
//         t.as_raw_fd(),
//         buf.as_ptr() as *const _,
//         buf.len() as _,
//         flags,
//         ep.as_ref() as *const _ as *const _,
//         ep.size() as _,
//     ) as ssize_t
// }

// pub fn setsockopt<T, P, C>(t: &T, pro: &P, cmd: C) -> io::Result<()>
// where
//     T: AsRawFd,
//     C: SetSocketOption<P>,
// {
//     libc_try!(ws2_32::setsockopt(
//         t.as_raw_fd(),
//         cmd.level(pro),
//         cmd.name(pro),
//         cmd.data() as *const _ as *const _,
//         cmd.size() as _,
//     ));
//     Ok(())
// }

// pub fn shutdown<T, H>(t: &T, how: H) -> io::Result<()>
// where
//     T: AsRawFd,
//     H: Into<i32>,
// {
//     libc_try!(ws2_32::shutdown(t.as_raw_fd(), how.into()));
//     Ok(())
// }

// pub fn socket<P>(pro: &P) -> io::Result<RawFd>
// where
//     P: Protocol,
// {
//     let s = unsafe { ws2_32::socket(pro.family_type(), pro.socket_type(), pro.protocol_type()) };
//     if s != INVALID_SOCKET {
//         Ok(s)
//     } else {
//         Err(io::Error::last_os_error())
//     }
// }

// pub fn startup() {
//     use winapi::WSADATA;

//     let mut wsa: WSADATA = unsafe { mem::uninitialized() };
//     libc_ign!(ws2_32::WSAStartup(2, &mut wsa));
// }

// pub unsafe fn write<T>(t: &T, buf: &[u8]) -> ssize_t
// where
//     T: AsRawFd,
// {
//     send(t, buf, 0)
// }

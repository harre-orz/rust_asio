use prelude::*;
use ffi::{RawFd, SystemError};
use libc;

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


fn init_fd(fd: RawFd) -> Result<(), SystemError> {
    unsafe {
        // FD_CLOEXEC
        let flags = libc::fcntl(fd, F_GETFD);
        if flags == -1 {
            return Err(SystemError::last_error())
        }
        if libc::fcntl(fd, F_SETFD, flags | FD_CLOEXEC) == -1 {
            return Err(SystemError::last_error())
        }

        // O_NONBLOCK
        let flags = libc::fcntl(fd, F_GETFL);
        if flags == -1 {
            return Err(SystemError::last_error())
        }
        if libc::fcntl(fd, F_SETFL, flags | O_NONBLOCK) == -1 {
            return Err(SystemError::last_error())
        }
    }
    Ok(())
}


pub fn accept<P, S>(soc: &S) -> Result<(RawFd, P::Endpoint), SystemError>
    where P: Protocol,
          S: Socket<P>,
{
    let mut ep = unsafe { soc.protocol().uninitialized() };
    let mut socklen = ep.capacity();
    match unsafe { libc::accept(soc.as_raw_fd(), ep.as_mut_ptr(), &mut socklen) } {
        -1 => Err(SystemError::last_error()),
        fd => unsafe {
            init_fd(fd)?;
            ep.resize(socklen);
            Ok((fd, ep))
        }
    }
}


pub fn bind<P, S>(soc: &S, ep: &P::Endpoint) -> Result<(), SystemError>
    where P: Protocol,
          S: Socket<P>,
{
    match unsafe { libc::bind(soc.as_raw_fd(), ep.as_ptr(), ep.size()) } {
        0 => Ok(()),
        _ => Err(SystemError::last_error()),
    }
}


#[cfg(debug_assertions)]
pub fn close(fd: RawFd) {
    if 0 != unsafe { libc::close(fd) } {
        panic!("{}", SystemError::last_error());
    }
}


#[cfg(not(debug_assertions))]
pub fn close(fd: RawFd) {
    unsafe { libc::close(fd) };
}


pub fn connect<P, S>(soc: &S, ep: &P::Endpoint) -> Result<(), SystemError>
    where P: Protocol,
          S: Socket<P>,
{
    match unsafe { libc::connect(soc.as_raw_fd(), ep.as_ptr(), ep.size()) } {
        0 => Ok(()),
        _ => Err(SystemError::last_error()),
    }
}


pub fn getpeername<P, S>(soc: &S) -> Result<P::Endpoint, SystemError>
    where P: Protocol,
          S: Socket<P>,
{
    let mut ep = unsafe { soc.protocol().uninitialized() };
    let mut socklen = ep.capacity();
    match unsafe { libc::getpeername(soc.as_raw_fd(), ep.as_mut_ptr(), &mut socklen) } {
        0 => {
            unsafe { ep.resize(socklen) };
            Ok(ep)
        },
        _ => Err(SystemError::last_error()),
    }
}


pub fn getsockname<P, S>(soc: &S) -> Result<P::Endpoint, SystemError>
    where P: Protocol,
          S: Socket<P>,
{
    let mut ep = unsafe { soc.protocol().uninitialized() };
    let mut socklen = ep.capacity();
    match unsafe { libc::getsockname(soc.as_raw_fd(), ep.as_mut_ptr(), &mut socklen) } {
        0 => {
            unsafe { ep.resize(socklen) };
            Ok(ep)
        },
        _ => Err(SystemError::last_error()),
    }
}

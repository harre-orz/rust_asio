pub use std::os::unix::io::{RawFd, AsRawFd};
pub use libc::{sockaddr};
use std::io;
use std::cmp;
use libc::{self, F_GETFL, F_SETFL, O_NONBLOCK, c_void, socklen_t};
use traits::{Protocol, SockAddr, IoControl, Shutdown, GetSocketOption, SetSocketOption};

mod io_ops;
pub use self::io_ops::*;

pub fn ioctl<T: AsRawFd, C: IoControl>(fd: &T, cmd: &mut C) -> io::Result<()> {
    libc_try!(libc::ioctl(fd.as_raw_fd(), cmd.name() as u64, cmd.data()));
    Ok(())
}

pub fn getflags<T: AsRawFd>(fd: &T) -> io::Result<i32> {
    Ok(libc_try!(libc::fcntl(fd.as_raw_fd(), F_GETFL)))
}

pub fn setflags<T: AsRawFd>(fd: &T, flags: i32) -> io::Result<()> {
    libc_try!(libc::fcntl(fd.as_raw_fd(), F_SETFL, flags));
    Ok(())
}

pub fn getnonblock<T: AsRawFd>(fd: &T) -> io::Result<bool> {
    Ok((try!(getflags(fd)) & libc::O_NONBLOCK) != 0)
}

pub fn setnonblock<T: AsRawFd>(fd: &T, on: bool) -> io::Result<()> {
    let flags = try!(getflags(fd));
    setflags(fd, if on { flags | O_NONBLOCK } else { flags & !O_NONBLOCK })
}

pub fn shutdown<T: AsRawFd>(fd: &T, how: Shutdown) -> io::Result<()> {
    libc_try!(libc::shutdown(fd.as_raw_fd(), how as i32));
    Ok(())
}

pub fn socket<P: Protocol>(pro: &P) -> io::Result<RawFd> {
    Ok(libc_try!(libc::socket(pro.family_type() as i32, pro.socket_type(), pro.protocol_type())))
}

pub fn bind<T: AsRawFd, E: SockAddr>(fd: &T, ep: &E) -> io::Result<()> {
    libc_try!(libc::bind(fd.as_raw_fd(), ep.as_sockaddr() as *const _ as *const sockaddr, ep.size() as libc::socklen_t));
    Ok(())
}

pub const SOMAXCONN: u32 = 126;
pub fn listen<T: AsRawFd>(fd: &T, backlog: u32) -> io::Result<()> {
    libc_try!(libc::listen(fd.as_raw_fd(), cmp::min(backlog, SOMAXCONN) as i32));
    Ok(())
}

pub fn getsockname<T: AsRawFd, E: SockAddr>(fd: &T, mut ep: E) -> io::Result<E> {
    let mut socklen = ep.capacity() as socklen_t;
    libc_try!(libc::getsockname(fd.as_raw_fd(), ep.as_mut_sockaddr() as *mut _ as *mut sockaddr, &mut socklen));
    unsafe { ep.resize(socklen as usize); }
    Ok(ep)
}

pub fn getpeername<T: AsRawFd, E: SockAddr>(fd: &T, mut ep: E) -> io::Result<E> {
    let mut socklen = ep.capacity() as socklen_t;
    libc_try!(libc::getpeername(fd.as_raw_fd(), ep.as_mut_sockaddr() as *mut _ as *mut sockaddr, &mut socklen));
    unsafe { ep.resize(socklen as usize); }
    Ok(ep)
}

pub fn getsockopt<T: AsRawFd, P: Protocol, C: GetSocketOption<P>>(fd: &T, pro: &P) -> io::Result<C> {
    let mut cmd = C::default();
    let mut datalen = 0;
    libc_try!(libc::getsockopt(fd.as_raw_fd(), cmd.level(pro), cmd.name(pro), cmd.data_mut() as *mut _ as *mut c_void, &mut datalen));
    cmd.resize(datalen as usize);
    Ok(cmd)
}

pub fn setsockopt<T: AsRawFd, P: Protocol, C: SetSocketOption<P>>(fd: &T, pro: &P, cmd: C) -> io::Result<()> {
    libc_try!(libc::setsockopt(fd.as_raw_fd(), cmd.level(pro), cmd.name(pro), cmd.data() as *const  _ as *const c_void, cmd.size() as socklen_t));
    Ok(())
}

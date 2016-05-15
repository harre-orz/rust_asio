extern crate libc;
extern crate time;
use std::io;
use std::mem;
use std::cmp;
use std::ptr;
use std::marker::PhantomData;
use time::Duration;

pub type NativeHandleType = i32;

pub enum Shutdown {
    Read, Write, Both,
}

pub trait IoObject<'a> : Sized {
    fn io_service(&self) -> &'a IoService;
}

pub trait ReadWrite<'a> : IoObject<'a> {
    fn read_some<B: MutableBuffer>(&self, buf: B) -> io::Result<usize>;
    fn write_some<B: Buffer>(&self, buf: B) -> io::Result<usize>;
}

#[derive(Default)]
pub struct IoService;

#[macro_use]
mod err;
use err::*;

mod pro;
pub use pro::*;

mod str;
pub use str::*;

mod buf;
pub use buf::*;

mod fun;
pub use fun::*;

pub mod ip;
pub mod local;

mod timers;
pub use timers::*;  // mod timer

mod options;
pub use options::*;  // mod option

type NativeSockAddrType = libc::sockaddr;

type NativeSockLenType = libc::socklen_t;

trait AsBytes {
    type Bytes;
    fn as_bytes(&self) -> &Self::Bytes;
    fn as_mut_bytes(&mut self) -> &mut Self::Bytes;
}

trait AsSockAddr {
    fn socklen(&self) -> NativeSockLenType;
    fn as_sockaddr(&self) -> &NativeSockAddrType;
    fn as_mut_sockaddr(&mut self) -> &mut NativeSockAddrType;
    fn eq_impl(&self, other: &Self) -> bool {
        unsafe {
            libc::memcmp(
                mem::transmute(self.as_sockaddr()),
                mem::transmute(other.as_sockaddr()),
                self.socklen() as usize
            ) == 0 }
    }
    fn cmp_impl(&self, other: &Self) -> cmp::Ordering {
        match unsafe {
            libc::memcmp(
                mem::transmute(self.as_sockaddr()),
                mem::transmute(other.as_sockaddr()),
                self.socklen() as usize
            ) }
        {
            0 => cmp::Ordering::Equal,
            x if x < 0 => cmp::Ordering::Less,
            _ => cmp::Ordering::Greater,
        }
    }
}

struct BasicSocket<'a, P: Protocol> {
    io: &'a IoService,
    fd: NativeHandleType,
    marker: PhantomData<P>,
}

const SOMAXCONN: i32 = 126;

impl<'a, P: Protocol> BasicSocket<'a, P> {
    fn io_service(&self) -> &'a IoService {
        self.io
    }

    unsafe fn native_handle(&self) -> &NativeHandleType {
        &self.fd
    }

    fn socket(io: &'a IoService, pro: P) -> io::Result<Self> {
        let fd = libc_try!(libc::socket(pro.family_type(), pro.socket_type() | libc::SOCK_CLOEXEC, pro.protocol_type()));
        Ok(BasicSocket { fd: fd, io: io, marker: PhantomData })
    }

    fn bind<E: Endpoint<P> + AsSockAddr>(io: &'a IoService, ep: &E) -> io::Result<Self> {
        let soc = try!(Self::socket(io, ep.protocol()));
        libc_try!(libc::bind(soc.fd, ep.as_sockaddr(), ep.socklen()));
        Ok(soc)
    }

    fn listen<E: Endpoint<P> + AsSockAddr>(io: &'a IoService, ep: &E) -> io::Result<Self> {
        let soc = try!(Self::socket(io, ep.protocol()));
        libc_try!(libc::bind(soc.fd, ep.as_sockaddr(), ep.socklen()));
        libc_try!(libc::listen(soc.fd, SOMAXCONN));
        Ok(soc)
    }

    fn connect<E: Endpoint<P> + AsSockAddr>(io: &'a IoService, ep: &E) -> io::Result<Self> {
        let soc = try!(BasicSocket::socket(io, ep.protocol()));

        let timeout = 0;
        try!(soc.set_non_blocking(true));
        if unsafe { libc::connect(soc.fd, ep.as_sockaddr(), ep.socklen()) == 0 } {
            let _ = soc.set_non_blocking(false);
            Ok(soc)
        } else if errno() != libc::EINPROGRESS {
            Err(io::Error::last_os_error())
        } else {
            try!(soc.ready(libc::POLLOUT, timeout));
            try!(soc.set_non_blocking(false));
            Ok(soc)
        }
    }

    fn reconnect<E: Endpoint<P> + AsSockAddr>(&self, ep: &E) -> io::Result<()> {
        libc_try!(libc::connect(self.fd, ep.as_sockaddr(), ep.socklen()));
        Ok(())
    }

    fn close(&self) -> io::Result<()> {
        libc_try!(libc::close(self.fd));
        Ok(())
    }

    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        let how = match how {
            Shutdown::Read => libc::SHUT_RD,
            Shutdown::Write => libc::SHUT_WR,
            Shutdown::Both => libc::SHUT_RDWR,
        };
        libc_try!(libc::shutdown(self.fd, how));
        Ok(())
    }

    fn accept<E: Endpoint<P> + AsSockAddr>(&self, mut ep: E) -> io::Result<(Self, E)> {
        let timeout = 0;
        try!(self.ready(libc::POLLIN, timeout));

        let mut socklen = ep.socklen();
        let fd = libc_try!(libc::accept(self.fd, ep.as_mut_sockaddr(), &mut socklen));
        Ok((BasicSocket { io: self.io, fd: fd, marker: PhantomData }, ep))
    }

    fn recv<B: MutableBuffer>(&self, mut buf: B, flags: i32) -> io::Result<usize> {
        let timeout = 0;
        try!(self.ready(libc::POLLIN, timeout));

        let size = libc_try!(libc::recv(self.fd, mem::transmute(&mut buf.as_mut_buffer()), buf.buffer_size(), flags));
        Ok(size as usize)
    }

    fn recv_from<B: MutableBuffer, E: Endpoint<P> + AsSockAddr>(&self, mut buf: B, flags: i32, mut ep: E) -> io::Result<(usize, E)> {
        let timeout = 0;
        try!(self.ready(libc::POLLIN, timeout));

        let mut socklen = ep.socklen();
        let size = libc_try!(libc::recvfrom(self.fd, mem::transmute(&mut buf.as_mut_buffer()), buf.buffer_size(), flags, ep.as_mut_sockaddr(), &mut socklen));
        Ok((size as usize, ep))
    }

    fn send<B: Buffer>(&self, buf: B, flags: i32) -> io::Result<usize> {
        let size = libc_try!(libc::send(self.fd, mem::transmute(&buf.as_buffer()), buf.buffer_size(), flags));
        Ok(size as usize)
    }

    fn send_to<B: Buffer, E: Endpoint<P> + AsSockAddr>(&self, buf: B, flags: i32, ep: &E) -> io::Result<usize> {
        let size = libc_try!(libc::sendto(self.fd, mem::transmute(&buf.as_buffer()), buf.buffer_size(), flags, ep.as_sockaddr(), ep.socklen()));
        Ok(size as usize)
    }

    fn local_endpoint<E: Endpoint<P> + AsSockAddr>(&self, mut ep: E) -> io::Result<E> {
        let mut socklen = ep.socklen();
        libc_try!(libc::getsockname(self.fd, ep.as_mut_sockaddr(), &mut socklen));
        Ok(ep)
    }

    fn remote_endpoint<E: Endpoint<P> + AsSockAddr>(&self, mut ep: E) -> io::Result<E> {
        let mut socklen = ep.socklen();
        libc_try!(libc::getpeername(self.fd, ep.as_mut_sockaddr(), &mut socklen));
        Ok(ep)
    }

    fn get_status_flags(&self) -> io::Result<i32> {
        Ok(libc_try!(libc::fcntl(self.fd, libc::F_GETFL)))
    }

    fn set_status_flags(&self, flags: i32) -> io::Result<()> {
        libc_try!(libc::fcntl(self.fd, libc::F_SETFL, flags));
        Ok(())
    }

    fn available(&self) -> io::Result<usize> {
        let mut cmd = option::Available(0);
        try!(self.io_control(&mut cmd));
        Ok(cmd.0 as usize)
    }

    fn get_non_blocking(&self) -> io::Result<bool> {
        Ok((try!(self.get_status_flags()) & libc::O_NONBLOCK) != 0)
    }

    fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        let flags = try!(self.get_status_flags());
        self.set_status_flags(if on { flags | libc::O_NONBLOCK } else { flags & !libc::O_NONBLOCK })
    }

    fn io_control<T: IoControl>(&self, cmd: &mut T) -> io::Result<()> {
        libc_try!(libc::ioctl(self.fd, cmd.name() as u64, cmd.data()));
        Ok(())
    }

    fn get_socket<T: GetSocketOption>(&self) -> io::Result<T> {
        let mut cmd = T::default();
        let mut datalen = 0;
        libc_try!(libc::getsockopt(self.fd, cmd.level(), cmd.name(), mem::transmute(cmd.data_mut()), &mut datalen));
        cmd.resize(datalen as usize);
        Ok(cmd)
    }

    fn set_socket<T: SetSocketOption>(&self, cmd: &T) -> io::Result<()> {
        libc_try!(libc::setsockopt(self.fd, cmd.level(), cmd.name(), mem::transmute(cmd.data()), cmd.size() as libc::socklen_t));
        Ok(())
    }

    fn ready(&self, op: i16, timeout: i32) -> io::Result<()> {
        let mut fd = libc::pollfd { fd: self.fd, events: op, revents: 0, };
        if libc_try!(libc::poll(mem::transmute(&mut fd), 1, timeout)) == 0 {
            Err(io::Error::new(io::ErrorKind::Other, "timed out"))
        } else {
            Ok(())
        }
    }
}

impl<'a, P: Protocol> Drop for BasicSocket<'a, P> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

struct BasicTimer<'a> {
    io: &'a IoService,
}

impl<'a> BasicTimer<'a> {
    fn io_service(&self) -> &'a IoService {
        self.io
    }

    fn wait(&self, time: &Duration) -> io::Result<()> {
        let mut tv = libc::timeval {
            tv_sec: time.num_seconds(),
            tv_usec: time.num_microseconds().unwrap_or(0) % 1000000,
        };
        libc_try!(libc::select(0, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), &mut tv));
        Ok(())
    }
}

impl<'a> Drop for BasicTimer<'a> {
    fn drop(&mut self) {
    }
}

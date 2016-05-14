extern crate libc;
use std::io;
use std::mem;
use std::cmp;
use std::fmt::Display;
use std::marker::PhantomData;

macro_rules! libc_try {
    ($expr:expr) => (match unsafe { $expr } {
        rc if rc >= 0 => rc,
        _ => return Err(io::Error::last_os_error()),
    })
}

extern {
    #[cfg_attr(target_os = "linux", link_name = "__errno_location")]
    fn errno_location() -> *mut libc::c_int;
}

fn errno() -> i32 {
    unsafe { *errno_location() }
}


pub type NativeHandleType = i32;

trait AsBytes {
    type Bytes;
    fn as_bytes(&self) -> &Self::Bytes;
    fn as_mut_bytes(&mut self) -> &mut Self::Bytes;
}

type NativeSockAddrType = libc::sockaddr;

type NativeSockLenType = libc::socklen_t;

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

#[derive(Default)]
pub struct IoService;

pub enum Shutdown {
    Read, Write, Both,
}

pub trait ReadWrite<'a> : IoObject<'a> {
    fn read_some<B: MutableBuffer>(&self, buf: B) -> io::Result<usize>;
    fn write_some<B: Buffer>(&self, buf: B) -> io::Result<usize>;
}

pub trait Protocol : Clone + Eq + PartialEq {
    fn family_type(&self) -> i32;
    fn socket_type(&self) -> i32;
    fn protocol_type(&self) -> i32;
}

pub trait Endpoint<P: Protocol> : Clone + Eq + PartialEq + Ord + PartialOrd + Display {
    fn protocol(&self) -> P;
}

pub trait IoObject<'a> : Sized {
    fn io_service(&self) -> &'a IoService;
}

pub trait SocketBase<P: Protocol> {
    type Endpoint : Endpoint<P>;
    unsafe fn native_handle(&self) -> &NativeHandleType;
    fn local_endpoint(&self) -> io::Result<Self::Endpoint>;
    fn io_control<T: IoControl>(&self, cmd: &mut T) -> io::Result<()>;
    fn get_socket<T: GetSocketOption>(&self) -> io::Result<T>;
    fn set_socket<T: SetSocketOption>(&self, cmd: &T) -> io::Result<()>;
    fn get_non_blocking(&self) -> io::Result<bool>;
    fn set_non_blocking(&self, on: bool) -> io::Result<()>;
}

pub trait Socket<'a, P: Protocol> : IoObject<'a> + SocketBase<P> {
    fn bind(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self>;
    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()>;
    fn remote_endpoint(&self) -> io::Result<Self::Endpoint>;
    fn available(&self) -> io::Result<usize>;
    fn recv<B: MutableBuffer>(&self, buf: B, flags: i32) -> io::Result<usize>;
    fn send<B: Buffer>(&self, buf: B, flags: i32) -> io::Result<usize>;
    fn recv_from<B: MutableBuffer>(&self, buf: B, flags: i32) -> io::Result<(usize, Self::Endpoint)>;
    fn send_to<B: Buffer>(&self, buf: B, flags: i32, ep: &Self::Endpoint) -> io::Result<usize>;
}

pub trait StreamSocket<'a, P: Protocol> : IoObject<'a> + SocketBase<P> {
    fn connect(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self>;
    fn shutdown(&self, how: Shutdown) -> io::Result<()>;
    fn remote_endpoint(&self) -> io::Result<Self::Endpoint>;
    fn available(&self) -> io::Result<usize>;
    fn recv<B: MutableBuffer>(&self, buf: B, flags: i32) -> io::Result<usize>;
    fn send<B: Buffer>(&self, buf: B, flags: i32) -> io::Result<usize>;
}

pub trait ListenerSocket<'a, P: Protocol> : IoObject<'a> + SocketBase<P> {
    type Socket : IoObject<'a> + SocketBase<P>;
    fn listen(io: &'a IoService, ep: &Self::Endpoint) -> io::Result<Self>;
    fn accept(&self) -> io::Result<(Self::Socket, Self::Endpoint)>;
}

mod str;

mod buf;
pub use buf::*;

mod cmd;
pub use cmd::*;

pub mod ip;
pub mod local;


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
        Ok(BasicSocket { fd: fd, io: io, marker: PhantomData, })
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
            soc.set_non_blocking(false);
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
        let mut cmd = base::Available(0);
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

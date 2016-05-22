use std::io;
use std::fmt;
use std::mem;
use {IoObject, IoService, Strand};
use ops::*;
use ops::async::*;

pub enum Shutdown {
    Read = SHUT_RD as isize,
    Write = SHUT_WR as isize,
    Both = SHUT_RDWR as isize,
}

pub trait Protocol : Clone + Eq + PartialEq {
    fn family_type(&self) -> i32;
    fn socket_type(&self) -> i32;
    fn protocol_type(&self) -> i32;
}

pub trait Endpoint<P: Protocol> : Clone + Eq + PartialEq + Ord + PartialOrd + fmt::Display {
    fn protocol(&self) -> P;
}

pub trait IoControl {
    type Data;
    fn name(&self) -> i32;
    fn data(&mut self) -> &mut Self::Data;
}

pub trait SocketOption : Default {
    type Data;
    fn level(&self) -> i32;
    fn name(&self) -> i32;
}

pub trait GetSocketOption : SocketOption {
    fn resize(&mut self, s: usize);
    fn data_mut(&mut self) -> &mut Self::Data;
}

pub trait SetSocketOption : SocketOption {
    fn size(&self) -> usize;
    fn data(&self) -> &Self::Data;
}

pub trait ReadWrite : Sized + AsRawFd {
    fn read_some(&self, buf: &mut [u8]) -> io::Result<usize>;

    fn async_read_some<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send;

    fn write_some(&self, buf: &[u8]) -> io::Result<usize>;

    fn async_write_some<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send;

    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self;
}

pub trait ResolveQuery<'a, P: Protocol> : Send {
    type Iter : Iterator;
    fn query(self, pro: P) -> io::Result<Self::Iter>;
}

pub trait Resolver<P: Protocol> : IoObject {
    fn new(io: &IoService) -> Self;
    fn resolve<'a, Q: ResolveQuery<'a, P>>(&self, query: Q) -> io::Result<Q::Iter>;
    fn async_resolve<'a, Q, A, F, T>(a: A, query: Q, callback: F, obj: &Strand<T>)
        where Q: ResolveQuery<'a, P>,
              A: Fn(&T) -> &Self + Send,
              F: FnOnce(&Strand<T>, io::Result<Q::Iter>) + Send;
}

pub trait SocketBase<P: Protocol> : IoObject + AsRawFd {
    type Endpoint : Endpoint<P>;

    fn new(io: &IoService, pro: P) -> io::Result<Self>;

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()>;

    fn local_endpoint(&self) -> io::Result<Self::Endpoint>;

    fn io_control<T: IoControl>(&self, cmd: &mut T) -> io::Result<()> {
        ioctl(self, cmd)
    }

    fn get_socket<T: GetSocketOption>(&self) -> io::Result<T> {
        self.io_service().interrupt();
        getsockopt(self)
    }

    fn set_socket<T: SetSocketOption>(&self, cmd: &T) -> io::Result<()> {
        setsockopt(self, cmd)
    }

    fn get_non_blocking(&self) -> io::Result<bool> {
        getnonblock(self)
    }

    fn set_non_blocking(&self, on: bool) -> io::Result<()> {
        setnonblock(self, on)
    }
}

pub trait DgramSocket<P: Protocol> : SocketBase<P> {
    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()>;

    fn async_connect<A, F, T>(a: A, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send,
              F: FnOnce(&Strand<T>, io::Result<()>) + Send;

    fn remote_endpoint(&self) -> io::Result<Self::Endpoint>;

    fn recv(&self, buf: &mut [u8], flags: i32) -> io::Result<usize>;

    fn async_recv<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send;

    fn recv_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, Self::Endpoint)>;

    fn async_recv_from<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<(usize, Self::Endpoint)>) + Send;

    fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize>;

    fn async_send<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send;

    fn send_to(&self, buf: &[u8], flags: i32, ep: &Self::Endpoint) -> io::Result<usize>;

    fn async_send_to<A, F, T>(a: A, flags: i32, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send;

    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self;

    fn available(&self) -> io::Result<usize> {
        let mut cmd = option::Available::default();
        try!(self.io_control(&mut cmd));
        Ok(cmd.0 as usize)
    }

    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        shutdown(self, how)
    }
}

pub trait StreamSocket<P: Protocol> : SocketBase<P> + ReadWrite {
    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()>;

    fn async_connect<A, F, T>(a: A, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send,
              F: FnOnce(&Strand<T>, io::Result<()>) + Send;

    fn remote_endpoint(&self) -> io::Result<Self::Endpoint>;

    fn recv(&self, buf: &mut [u8], flags: i32) -> io::Result<usize>;

    fn async_recv<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send;

    fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize>;

    fn async_send<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send;

    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self;

    fn available(&self) -> io::Result<usize> {
        let mut cmd = option::Available::default();
        try!(self.io_control(&mut cmd));
        Ok(cmd.0 as usize)
    }

    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        shutdown(self, how)
    }
}

pub trait SocketListener<P: Protocol> : SocketBase<P> {
    type Socket : SocketBase<P>;

    fn listen(&self) -> io::Result<()> {
        listen(self, SOMAXCONN)
    }

    fn accept(&self) -> io::Result<(Self::Socket, Self::Endpoint)>;

    fn async_accept<A, F, T>(a: A, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send,
              F: FnOnce(&Strand<T>, io::Result<(Self::Socket, Self::Endpoint)>) + Send;

    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self;
}

pub trait RawSocket<P: Protocol> : SocketBase<P> {
    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()>;

    fn async_connect<A, F, T>(a: A, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send,
              F: FnOnce(&Strand<T>, io::Result<()>) + Send;

    fn remote_endpoint(&self) -> io::Result<Self::Endpoint>;

    fn recv(&self, buf: &mut [u8], flags: i32) -> io::Result<usize>;

    fn async_recv<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send;

    fn recv_from(&self, buf: &mut [u8], flags: i32) -> io::Result<(usize, Self::Endpoint)>;

    fn async_recv_from<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<(usize, Self::Endpoint)>) + Send;

    fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize>;

    fn async_send<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send;

    fn send_to(&self, buf: &[u8], flags: i32, ep: &Self::Endpoint) -> io::Result<usize>;

    fn async_send_to<A, F, T>(a: A, flags: i32, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send;

    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self;

    fn available(&self) -> io::Result<usize> {
        let mut cmd = option::Available::default();
        try!(self.io_control(&mut cmd));
        Ok(cmd.0 as usize)
    }

    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        shutdown(self, how)
    }
}

pub trait SeqPacketSocket<P: Protocol> : SocketBase<P> {
    fn connect(&self, ep: &Self::Endpoint) -> io::Result<()>;

    fn async_connect<A, F, T>(a: A, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send,
              F: FnOnce(&Strand<T>, io::Result<()>) + Send;

    fn remote_endpoint(&self) -> io::Result<Self::Endpoint>;

    fn recv(&self, buf: &mut [u8], flags: i32) -> io::Result<usize>;

    fn async_recv<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send;

    fn send(&self, buf: &[u8], flags: i32) -> io::Result<usize>;

    fn async_send<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send,
              F: FnOnce(&Strand<T>, io::Result<usize>) + Send;

    fn cancel<A, T>(a: A, obj: &Strand<T>)
        where A: Fn(&T) -> &Self;

    fn available(&self) -> io::Result<usize> {
        let mut cmd = option::Available::default();
        try!(self.io_control(&mut cmd));
        Ok(cmd.0 as usize)
    }

    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        shutdown(self, how)
    }
}

pub mod option;
pub mod local;
pub mod ip;

mod buf;
pub use self::buf::*;
mod fun;
pub use self::fun::*;

use std::io;
use std::fmt;
use std::mem;
use {IoObject, Strand, Cancel};
use ops::*;

/// Possible values which can be passed to the shutdown method.
pub enum Shutdown {
    /// Indicates that the reading portion of this socket should be shut down.
    Read = SHUT_RD as isize,

    /// Indicates that the writing portion of this socket should be shut down.
    Write = SHUT_WR as isize,

    /// Shut down both the reading and writing portions of this socket.
    Both = SHUT_RDWR as isize,
}

pub trait Protocol : Clone + Eq + PartialEq {
    /// Returns a value suitable for passing as the domain argument.
    fn family_type(&self) -> i32;

    /// Returns a value suitable for passing as the type argument.
    fn socket_type(&self) -> i32;

    /// Returns a value suitable for passing as the protocol argument.
    fn protocol_type(&self) -> i32;
}

pub trait Endpoint<P: Protocol> : Clone + Eq + PartialEq + Ord + PartialOrd + fmt::Display {
    fn protocol(&self) -> P;
}

pub trait Socket : Sized + AsRawFd {
    type Protocol : Protocol;
    type Endpoint : Endpoint<Self::Protocol>;

    fn bind(&self, ep: &Self::Endpoint) -> io::Result<()>;

    fn local_endpoint(&self) -> io::Result<Self::Endpoint>;

    fn io_control<T: IoControl<Self>>(&self, cmd: &mut T) -> io::Result<()> {
        ioctl(self, cmd)
    }

    fn get_option<T: GetSocketOption<Self>>(&self) -> io::Result<T> {
        getsockopt(self)
    }

    fn set_option<T: SetSocketOption<Self>>(&self, cmd: &T) -> io::Result<()> {
        setsockopt(self, cmd)
    }
}

pub trait SocketConnector : Socket + Cancel {
    fn connect<T: IoObject>(&self, io: &T, ep: &Self::Endpoint) -> io::Result<()>;

    fn async_connect<A, F, T>(a: A, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send,
              F: FnOnce(Strand<T>, io::Result<()>) + Send;

    fn remote_endpoint(&self) -> io::Result<Self::Endpoint>;

    fn available(&self) -> io::Result<usize> {
        let mut cmd = Available::default();
        try!(self.io_control(&mut cmd));
        Ok(cmd.0 as usize)
    }
}

pub trait SocketListener : Socket + Cancel {
    type Socket : Socket;

    fn listen(&self) -> io::Result<()> {
        listen(self, SOMAXCONN)
    }

    fn accept<T: IoObject>(&self, io: &T) -> io::Result<(Self::Socket, Self::Endpoint)>;

    fn async_accept<A, F, T>(a: A, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> &Self + Send,
              F: FnOnce(Strand<T>, io::Result<(Self::Socket, Self::Endpoint)>) + Send;
}

pub trait StreamSocket : SocketConnector + SendRecv + ReadWrite {
    fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        shutdown(self, how)
    }
}

pub trait DgramSocket : SocketConnector + SendRecv + SendToRecvFrom {
}

pub trait RawSocket : SocketConnector + SendRecv + SendToRecvFrom {
}

pub trait NonBlocking : Sized + AsRawFd {
    fn get_non_blocking(&self) -> bool;

    fn set_non_blocking(&self, on: bool);

    fn native_get_non_blocking(&self) -> io::Result<bool> {
        getnonblock(self)
    }

    fn native_set_non_blocking(&self, on: bool) -> io::Result<()> {
        setnonblock(self, on)
    }
}

pub trait SendRecv : Socket + Cancel {
    fn send<T: IoObject>(&self, io: &T, buf: &[u8], flags: i32) -> io::Result<usize>;

    fn async_send<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send;

    fn recv<T: IoObject>(&self, io: &T, buf: &mut [u8], flags: i32) -> io::Result<usize>;

    fn async_recv<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send;
}

pub trait SendToRecvFrom : Socket + Cancel {
    fn send_to<T: IoObject>(&self, io: &T, buf: &[u8], flags: i32, ep: &Self::Endpoint) -> io::Result<usize>;

    fn async_send_to<A, F, T>(a: A, flags: i32, ep: &Self::Endpoint, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send;

    fn recv_from<T: IoObject>(&self, io: &T, buf: &mut [u8], flags: i32) -> io::Result<(usize, Self::Endpoint)>;

    fn async_recv_from<A, F, T>(a: A, flags: i32, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send,
              F: FnOnce(Strand<T>, io::Result<(usize, Self::Endpoint)>) + Send;
}

pub trait ReadWrite : Sized + Cancel {
    fn read_some<T: IoObject>(&self, io: &T, buf: &mut [u8]) -> io::Result<usize>;

    fn async_read_some<A, F, T>(a: A, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut [u8]) + Send,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send;

    fn write_some<T: IoObject>(&self, io: &T, buf: &[u8]) -> io::Result<usize>;

    fn async_write_some<A, F, T>(a: A, callback: F, obj: &Strand<T>)
        where A: Fn(&T) -> (&Self, &[u8]) + Send,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send;

    // fn read_until<T: IoObject, C: MatchCondition>(&self, io: &T, sbuf: &mut StreamBuf, cond: C) -> io::Result<usize> {
    //     read_until(self, sbuf, cond)
    // }

    fn async_read_until<A, C, F, T>(a: A, cond: C, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut StreamBuf) + Send,
              C: MatchCondition + Send,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send;

    // fn write_until<T: IoObject, C: MatchCondition>(&self, io: &T, sbuf: &mut StreamBuf, cond: C) -> io::Result<usize> {
    //     write_until(self, sbuf, cond)
    // }

    fn async_write_until<A, C, F, T>(a: A, cond: C, callback: F, obj: &Strand<T>)
        where A: Fn(&mut T) -> (&Self, &mut StreamBuf) + Send,
              C: MatchCondition + Send,
              F: FnOnce(Strand<T>, io::Result<usize>) + Send {
        unimplemented!();
    }
}

pub trait IoControl<S: Socket> {
    type Data : Sized;
    fn name(&self) -> i32;
    fn data(&mut self) -> &mut Self::Data;
}

pub trait GetSocketOption<S: Socket> : Default {
    type Data;
    fn level(&self) -> i32;
    fn name(&self) -> i32;

    fn size(&self) -> usize {
        mem::size_of::<Self::Data>()
    }

    fn resize(&mut self, _: usize) {
    }

    fn data_mut(&mut self) -> &mut Self::Data;
}

pub trait SetSocketOption<S: Socket> : GetSocketOption<S> {
    fn data(&self) -> &Self::Data;
}

#[derive(Default, Clone)]
struct Available(pub i32);

impl<S: Socket> IoControl<S> for Available {
    type Data = i32;

    fn name(&self) -> i32 {
        FIONREAD as i32
    }

    fn data(&mut self) -> &mut i32 {
        &mut self.0
    }
}

pub mod socket_base;
pub mod local;
pub mod ip;

mod buf;
pub use self::buf::*;
mod fun;
pub use self::fun::*;

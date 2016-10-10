use std::mem;
use std::os::unix::io::{RawFd, AsRawFd};
use libc::{SHUT_RD, SHUT_WR, SHUT_RDWR, sockaddr};
use io_service::{IoObject, IoService};

/// Possible values which can be passed to the shutdown method.
pub enum Shutdown {
    /// Indicates that the reading portion of this socket should be shut down.
    Read = SHUT_RD as isize,

    /// Indicates that the writing portion of this socket should be shut down.
    Write = SHUT_WR as isize,

    /// Shut down both the reading and writing portions of this socket.
    Both = SHUT_RDWR as isize,
}

pub trait SockAddr : Clone + Send + 'static {
    fn as_sockaddr(&self) -> &sockaddr;

    fn as_mut_sockaddr(&mut self) -> &mut sockaddr;

    fn capacity(&self) -> usize;

    fn size(&self) -> usize;

    unsafe fn resize(&mut self, size: usize);
}

pub trait Endpoint<P> : SockAddr {
    fn protocol(&self) -> P;
}

pub trait Protocol : Clone + Send + 'static {
    type Endpoint : Endpoint<Self>;

    /// Reurns a value suitable for passing as the domain argument.
    fn family_type(&self) -> i32;

    /// Returns a value suitable for passing as the type argument.
    fn socket_type(&self) -> i32;

    /// Returns a value suitable for passing as the protocol argument.
    fn protocol_type(&self) -> i32;

    unsafe fn uninitialized(&self) -> Self::Endpoint;
}

pub trait IoControl {
    type Data;

    fn name(&self) -> i32;

    fn data(&mut self) -> &mut Self::Data;
}

pub trait SocketOption<P: Protocol> {
    type Data;

    fn level(&self, pro: &P) -> i32;

    fn name(&self, pro: &P) -> i32;
}

pub trait GetSocketOption<P: Protocol> : SocketOption<P> + Default {
    fn data_mut(&mut self) -> &mut Self::Data;

    fn resize(&mut self, _size: usize) {
    }
}

pub trait SetSocketOption<P: Protocol> : SocketOption<P> {
    fn data(&self) -> &Self::Data;

    fn size(&self)  -> usize {
        mem::size_of::<Self::Data>()
    }
}

#[doc(hidden)]
pub trait FromRawFd<P: Protocol> : AsRawFd + Send + 'static {
    unsafe fn from_raw_fd<T: IoObject>(io: &T, pro: P, fd: RawFd) -> Self;
}

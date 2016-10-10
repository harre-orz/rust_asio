use std::mem;
use libc::{SHUT_RD, SHUT_WR, SHUT_RDWR, sockaddr};

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

    #[doc(hidden)]
    unsafe fn uninitialized(&self) -> Self::Endpoint;
}

pub trait IoControl {
    type Data;

    fn name(&self) -> i32;

    fn data(&mut self) -> &mut Self::Data;
}

pub trait SocketOption<P> {
    type Data;

    fn level(&self, pro: &P) -> i32;

    fn name(&self, pro: &P) -> i32;
}

pub trait GetSocketOption<P> : SocketOption<P> + Default {
    fn data_mut(&mut self) -> &mut Self::Data;

    fn resize(&mut self, _size: usize) {
    }
}

pub trait SetSocketOption<P> : SocketOption<P> {
    fn data(&self) -> &Self::Data;

    fn size(&self)  -> usize {
        mem::size_of::<Self::Data>()
    }
}

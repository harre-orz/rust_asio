use std::fmt;
use std::mem;

pub trait SockAddr : Clone + Send + 'static {
    type SockAddr : ?Sized;

    fn as_ref(&self) -> &Self::SockAddr;

    unsafe fn as_mut(&mut self) -> &mut Self::SockAddr;

    fn capacity(&self) -> usize;

    fn size(&self) -> usize;

    unsafe fn resize(&mut self, size: usize);
}

pub trait Endpoint<P> : SockAddr {
    fn protocol(&self) -> P;
}

pub trait Protocol : fmt::Debug + Clone + Send + 'static {
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

    #[cfg(unix)] fn name(&self) -> u64;
    #[cfg(windows)] fn name(&self) -> i32;

    fn data(&mut self) -> &mut Self::Data;
}

pub trait SocketOption<P> {
    type Data;

    fn level(&self, pro: &P) -> i32;

    fn name(&self, pro: &P) -> i32;
}

pub trait GetSocketOption<P> : SocketOption<P> + Default {
    fn capacity(&self) -> usize {
        mem::size_of::<Self::Data>()
    }

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

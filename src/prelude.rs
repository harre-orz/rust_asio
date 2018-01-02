use ffi::{RawFd, AsRawFd, sockaddr, socklen_t, c_void};
use core::{IoContext, ThreadIoContext, Yield};

use std::io;


pub trait Endpoint<P> : Clone + Eq + Ord + Send + 'static {
    fn protocol(&self) -> P;

    fn as_ptr(&self) -> *const sockaddr;

    fn as_mut_ptr(&mut self) -> *mut sockaddr;

    fn capacity(&self) -> socklen_t;

    fn size(&self) -> socklen_t;

    unsafe fn resize(&mut self, len: socklen_t);
}


pub trait Protocol : Copy + Eq + Ord + Send + 'static {
    type Endpoint : Endpoint<Self>;

    /// Reurns a value suitable for passing as the domain argument.
    fn family_type(&self) -> i32;

    /// Returns a value suitable for passing as the type argument.
    fn socket_type(&self) -> i32;

    /// Returns a value suitable for passing as the protocol argument.
    fn protocol_type(&self) -> i32;

    unsafe fn uninitialized(&self) -> Self::Endpoint;
}


pub trait Socket<P> : AsRawFd + Send + 'static {
    /// Returns a socket protocol type.
    fn protocol(&self) -> &P;

    unsafe fn from_raw_fd(ctx: &IoContext, pro: P, fd: RawFd) -> Self;
}


pub trait IoControl : Sized {
    fn name(&self) -> u64;

    fn as_mut_ptr(&mut self) -> *mut c_void {
        self as *mut _ as *mut _
    }
}


pub trait SocketOption<P> : Sized {
    fn level(&self, pro: &P) -> i32;

    fn name(&self, pro: &P) -> i32;

    fn capacity(&self) -> u32 {
        use std::mem;
        mem::size_of::<Self>() as u32
    }
}


pub trait GetSocketOption<P> : SocketOption<P> + Default {
    fn as_mut_ptr(&mut self) -> *mut c_void {
        self as *mut _ as *mut _
    }

    unsafe fn resize(&mut self, _len: u32) {
    }
}


pub trait SetSocketOption<P> : SocketOption<P> {
    fn as_ptr(&self) -> *const c_void {
        self as *const _ as *const _
    }

    fn size(&self) -> u32 {
        self.capacity()
    }
}

pub trait Handler<R, E> {
    type Output;

    #[doc(hidden)]
    type Perform;

    #[doc(hidden)]
    type Yield : Yield<Self::Output>;

    #[doc(hidden)]
    fn channel(self) -> (Self::Perform, Self::Yield);

    #[doc(hidden)]
    fn complete(self, this: &mut ThreadIoContext, res: Result<R, E>);

    #[doc(hidden)]
    fn success(self: Box<Self>, this: &mut ThreadIoContext, res: R);

    #[doc(hidden)]
    fn failure(self: Box<Self>, this: &mut ThreadIoContext, err: E);
}


pub trait Stream : io::Read + io::Write {
    fn async_read_some<F>(&self, buf: &mut [u8], handler: F) -> F::Output
        where F: Handler<usize, io::Error>;

    fn async_write_some<F>(&self, buf: &mut [u8], handler: F) -> io::Result<usize>
        where F: Handler<usize, io::Error>;
}

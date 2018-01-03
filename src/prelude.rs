use ffi::{SystemError, RawFd, AsRawFd, sockaddr, socklen_t, c_void};
use core::{IoContext, ThreadIoContext, Perform};


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

    unsafe fn from_raw_fd(ctx: &IoContext, soc: RawFd, pro: P) -> Self;

    #[doc(hidden)]
    fn add_read_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    #[doc(hidden)]
    fn add_write_op(&self, this: &mut ThreadIoContext, op: Box<Perform>, err: SystemError);

    #[doc(hidden)]
    fn cancel_read_ops(&self, this: &mut ThreadIoContext);

    #[doc(hidden)]
    fn cancel_write_ops(&self, this: &mut ThreadIoContext);

    #[doc(hidden)]
    fn next_read_op(&self, this: &mut ThreadIoContext);

    #[doc(hidden)]
    fn next_write_op(&self, this: &mut ThreadIoContext);
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

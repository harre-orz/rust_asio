use ffi::AsRawFd;
use error::ErrCode;
use core::{AsIoContext, AsyncFd, FnOp, ThreadIoContext, workplace};

use std::io;

pub trait AsAsyncFd : AsIoContext + AsRawFd + 'static {
    fn as_fd(&self) -> &AsyncFd;
}

pub trait AsyncInput : AsAsyncFd {
    fn add_op<T>(&self, this: &mut ThreadIoContext, op: T, ec: ErrCode)
        where T: Into<Box<FnOp + Send>>
    {
        self.as_fd().add_input_op(this, op.into(), ec)
    }

    fn next_op(&self, this: &mut ThreadIoContext) {
        self.as_fd().next_input_op(this)
    }
}

impl<T: AsAsyncFd> AsyncInput for T { }

pub trait AsyncOutput : AsAsyncFd {
    fn add_op<T>(&self, this: &mut ThreadIoContext, op: T, ec: ErrCode)
        where T: Into<Box<FnOp + Send>>
    {
        self.as_fd().add_output_op(this, op.into(), ec)
    }

    fn next_op(&self, this: &mut ThreadIoContext) {
        self.as_fd().next_output_op(this)
    }
}

impl<T: AsAsyncFd> AsyncOutput for T { }

pub fn cancel<T>(t: &T)
    where T: AsAsyncFd,
{
    workplace(t.as_ctx(), |this| {
        t.as_fd().cancel_input_op(this);
        t.as_fd().cancel_output_op(this);
    })
}

pub fn getnonblock<T>(t: &T) -> io::Result<bool>
    where T: AsAsyncFd,
{
    t.as_fd().getnonblock()
}

pub fn setnonblock<T>(t: &T, on: bool) -> io::Result<()>
    where T: AsAsyncFd,
{
    t.as_fd().setnonblock(on)
}

mod connect;
pub use self::connect::*;

mod accept;
pub use self::accept::*;

mod read;
pub use self::read::*;

mod write;
pub use self::write::*;

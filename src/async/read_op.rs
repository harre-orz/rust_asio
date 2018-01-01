use ffi;
use ffi::posix::recv;
use prelude::{Protocol};
use core::{AsIoContext, ThreadIoContext, SocketImpl, Task, ReadOp, ErrCode};
use super::{Handler, NoYield};


use std::io;
use libc;


pub struct AsyncReceive<P, F> {
    pub socket: *const SocketImpl<P>,
    pub buffer: *const u8,
    pub buflen: usize,
    pub handler: F,
    pub errcode: ErrCode,
}

unsafe impl<P, F> Send for AsyncReceive<P, F> {}
unsafe impl<P, F> Sync for AsyncReceive<P, F> {}


impl<P: Protocol, F: Handler<usize, io::Error> > Handler<usize, io::Error> for AsyncReceive<P,F> {
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }

    fn complete(self, this: &mut ThreadIoContext, res: Result<usize, io::Error>) {
    }

    fn success(self: Box<Self>, this: &mut ThreadIoContext, res: usize) {
        self.handler.complete(this, Ok(res))

    }

    fn failure(self: Box<Self>, this: &mut ThreadIoContext, err: io::Error) {
        async_read_op(this, *self)
    }
}


impl<P: Protocol, F> Task for AsyncReceive<P, F>
    where F: Handler<usize, io::Error>
{
    fn call(self, this: &mut ThreadIoContext) {
        async_read_op(this, self)
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }
}

impl<P: Protocol, F> ReadOp for AsyncReceive<P,F>
    where F: Handler<usize, io::Error>
{
    fn read_op(self: Box<Self>, this: &mut ThreadIoContext, ec: ErrCode) {
        let soc = unsafe { &*self.socket };
        let mut buf = [0; 16];
        ffi::recv(soc, &mut buf, 0);
        self.success(this, 0);
    }
}

pub fn async_read_op<P: Protocol, F>(this: &mut ThreadIoContext, handler: AsyncReceive<P,F>)
    where F: Handler<usize, io::Error>
{
    let soc = unsafe { &*handler.socket };
    let ec = handler.errcode.clone();
    soc.register_read_op(this, box handler, ec);
}

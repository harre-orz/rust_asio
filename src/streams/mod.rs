use prelude::{Protocol, Socket};
use core::{AsIoContext, ThreadIoContext, Task};
use async::{Handler, Complete, Yield};

use std::io;
use std::marker::PhantomData;

mod sbuf;
pub use self::sbuf::*;

mod cond;
pub use self::cond::*;

mod read_until_op;
pub use self::read_until_op::*;

mod write_at_op;
pub use self::write_at_op::*;


struct ErrorHandler<F, R, E>(F, E, PhantomData<R>);

impl<F, R, E> Task for ErrorHandler<F, R, E>
    where F: Complete<R, E>,
          R: Send + 'static,
          E: Send + 'static,
{
    fn call(self, this: &mut ThreadIoContext) {
        let ErrorHandler(handler, err, _marker) = self;
        handler.failure(this, err)
    }

    fn call_box(self: Box<Self>, this: &mut ThreadIoContext) {
        self.call(this)
    }
}


pub trait Stream<P> : Socket<P> + AsIoContext + io::Read + io::Write + Sized
    where P: Protocol,
{
    fn async_read_some<F>(&self, buf: &mut [u8], handler: F) -> F::Output
        where F: Handler<usize, io::Error>;

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
        where F: Handler<usize, io::Error>;

    fn async_read_to_end<F>(&self, sbuf: &mut StreamBuf, handler: F) -> F::Output
        where F: Handler<usize, io::Error>
    {
        let (tx, rx) = handler.channel();
        let sbuf_ptr = sbuf as *mut _;
        match sbuf.prepare(4096) {
            Ok(buf) =>
                self.async_read_some(buf, AsyncReadToEnd::new(self, sbuf_ptr, tx)),
            Err(err) =>
                self.as_ctx().do_dispatch(ErrorHandler(tx, err, PhantomData)),
        }
        rx.yield_return(self.as_ctx())
    }

    fn async_read_until<M, F>(&self, sbuf: &mut StreamBuf, cond: M, handler: F) -> F::Output
        where M: MatchCond,
              F: Handler<usize, io::Error>
    {
        let (tx, rx) = handler.channel();
        let sbuf_ptr = sbuf as *mut _;
        match sbuf.prepare(4096) {
            Ok(buf) =>
                self.async_read_some(buf, AsyncReadUntil::new(self, sbuf_ptr, cond, tx)),
            Err(err) =>
                self.as_ctx().do_dispatch(ErrorHandler(tx, err, PhantomData)),
        }
        rx.yield_return(self.as_ctx())
    }

    fn async_write_all<M, F>(&self, sbuf: &mut StreamBuf, handler: F) -> F::Output
        where M: MatchCond,
              F: Handler<usize, io::Error>,
    {
        let (tx, rx) = handler.channel();
        let sbuf_ptr = sbuf as *mut _;
        let buf = sbuf.as_bytes();
        let len = buf.len();
        self.async_write_some(buf, AsyncWriteAt::new(self, sbuf_ptr, len, tx));
        rx.yield_return(self.as_ctx())
    }

    fn async_write_until<M, F>(&self, sbuf: &mut StreamBuf, mut cond: M, handler: F) -> F::Output
        where M: MatchCond,
              F: Handler<usize, io::Error>,
    {
        let (tx, rx) = handler.channel();
        let sbuf_ptr = sbuf as *mut _;
        let buf = sbuf.as_bytes();
        let len = cond.match_cond(buf).unwrap_or(buf.len());
        self.async_write_some(buf, AsyncWriteAt::new(self, sbuf_ptr, len, tx));
        rx.yield_return(self.as_ctx())
    }
}

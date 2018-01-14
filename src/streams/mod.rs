use core::AsIoContext;
use async::{Handler, Yield};

use std::io;


mod sbuf;
pub use self::sbuf::*;

mod cond;
pub use self::cond::*;

mod stream_op;
pub use self::stream_op::*;


pub trait Stream: AsIoContext + io::Read + io::Write + Sized + Send + 'static {
    fn async_read_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, io::Error>;

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, io::Error>;

    fn async_read_to_end<F>(&self, sbuf: &mut StreamBuf, handler: F) -> F::Output
    where
        F: Handler<usize, io::Error>,
    {
        let (tx, rx) = handler.channel();
        let sbuf_ptr = sbuf as *mut _;
        match sbuf.prepare(4096) {
            Ok(buf) => self.async_read_some(buf, AsyncReadToEnd::new(self, sbuf_ptr, tx)),
            Err(err) => self.as_ctx().do_dispatch(ErrorHandler::new(tx, err)),
        }
        rx.yield_return()
    }

    fn async_read_until<M, F>(&self, sbuf: &mut StreamBuf, cond: M, handler: F) -> F::Output
    where
        M: MatchCond,
        F: Handler<usize, io::Error>,
    {
        let (tx, rx) = handler.channel();
        let sbuf_ptr = sbuf as *mut _;
        match sbuf.prepare(4096) {
            Ok(buf) => self.async_read_some(buf, AsyncReadUntil::new(self, sbuf_ptr, cond, tx)),
            Err(err) => self.as_ctx().do_dispatch(ErrorHandler::new(tx, err)),
        }
        rx.yield_return()
    }

    fn async_write_all<M, F>(&self, sbuf: &mut StreamBuf, handler: F) -> F::Output
    where
        M: MatchCond,
        F: Handler<usize, io::Error>,
    {
        let (tx, rx) = handler.channel();
        let sbuf_ptr = sbuf as *mut _;
        let buf = sbuf.as_bytes();
        let len = buf.len();
        self.async_write_some(buf, AsyncWriteAt::new(self, sbuf_ptr, len, tx));
        rx.yield_return()
    }

    fn async_write_until<M, F>(&self, sbuf: &mut StreamBuf, mut cond: M, handler: F) -> F::Output
    where
        M: MatchCond,
        F: Handler<usize, io::Error>,
    {
        let (tx, rx) = handler.channel();
        let sbuf_ptr = sbuf as *mut _;
        let buf = sbuf.as_bytes();
        let len = cond.match_cond(buf).unwrap_or(buf.len());
        self.async_write_some(buf, AsyncWriteAt::new(self, sbuf_ptr, len, tx));
        rx.yield_return()
    }
}

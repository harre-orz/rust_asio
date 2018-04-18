use ffi::Timeout;
use core::{IoContext, AsIoContext, ThreadIoContext, Cancel};
use streambuf::{StreamBuf, MatchCond};
use handler::{Handler, Complete, Failure};

use std::io;

struct AsyncReadToEnd<F, S> {
    soc: *const S,
    sbuf: *mut StreamBuf,
    len: usize,
    handler: F,
}

unsafe impl<F, S> Send for AsyncReadToEnd<F, S> {}

impl<F, S> Handler<usize, S::Error> for AsyncReadToEnd<F, S>
where
    F: Complete<usize, S::Error>,
    S: Stream,
{
    type Output = ();

    type Handler = Self;

    fn wrap<C, W>(self, ctx: &C, wrapper: W) -> Self::Output
    where
        C: AsIoContext,
        W: FnOnce(&IoContext, Self::Handler),
    {
        wrapper(ctx.as_ctx(), self)
    }

    // fn wrap_timeout<C, W>(self, ctx: &C, _: Timeout, wrapper: W) -> Self::Output
    //     where C: Cancel,
    //           W: FnOnce(&IoContext, Self::Handler)
    // {
    //     wrapper(ctx.as_ctx(), self)
    // }
}

impl<F, S> Complete<usize, S::Error> for AsyncReadToEnd<F, S>
where
    F: Complete<usize, S::Error>,
    S: Stream,
{
    fn success(mut self, this: &mut ThreadIoContext, len: usize) {
        self.len += len;
        let soc = unsafe { &*self.soc };
        let sbuf = unsafe { &mut *self.sbuf };
        match sbuf.prepare(4096) {
            Ok(buf) => soc.async_read_some(buf, self),
            Err(err) => self.handler.failure(this, err.into()),
        }
    }

    fn failure(self, this: &mut ThreadIoContext, err: S::Error) {
        if self.len > 0 {
            self.handler.success(this, self.len)
        } else {
            self.handler.failure(this, err)
        }
    }
}

struct AsyncReadUntil<F, S, M> {
    soc: *const S,
    sbuf: *mut StreamBuf,
    cur: usize,
    cond: M,
    handler: F,
}

unsafe impl<F, S, M> Send for AsyncReadUntil<F, S, M> {}

impl<F, S, M> Handler<usize, S::Error> for AsyncReadUntil<F, S, M>
where
    F: Complete<usize, S::Error>,
    S: Stream,
    M: MatchCond,
{
    type Output = ();

    type Handler = Self;

    fn wrap<C, W>(self, ctx: &C, wrapper: W) -> Self::Output
    where
        C: AsIoContext,
        W: FnOnce(&IoContext, Self::Handler),
    {
        wrapper(ctx.as_ctx(), self)
    }

    // fn wrap_timeout<C, W>(self, ctx: &C, _: Timeout, wrapper: W) -> Self::Output
    //     where C: Cancel,
    //           W: FnOnce(&IoContext, Self::Handler)
    // {
    //     wrapper(ctx.as_ctx(), self)
    // }
}

impl<F, S, M> Complete<usize, S::Error> for AsyncReadUntil<F, S, M>
where
    F: Complete<usize, S::Error>,
    S: Stream,
    M: MatchCond,
{
    fn success(mut self, this: &mut ThreadIoContext, len: usize) {
        let soc = unsafe { &*self.soc };
        let sbuf = unsafe { &mut *self.sbuf };
        let cur = self.cur;
        sbuf.commit(len);
        match self.cond.match_cond(&sbuf.as_bytes()[cur..]) {
            Ok(len) => self.handler.success(this, cur + len),
            Err(len) => {
                match sbuf.prepare(4096) {
                    Ok(buf) => {
                        self.cur += len;
                        soc.async_read_some(buf, self)
                    }
                    Err(err) => self.failure(this, err.into()),
                }
            }
        }
    }

    fn failure(self, this: &mut ThreadIoContext, err: S::Error) {
        self.handler.failure(this, err)
    }
}

struct AsyncWriteAt<F, S> {
    soc: *const S,
    sbuf: *mut StreamBuf,
    len: usize,
    left: usize,
    handler: F,
}

unsafe impl<F, S> Send for AsyncWriteAt<F, S> {}

impl<F, S> Handler<usize, S::Error> for AsyncWriteAt<F, S>
where
    F: Complete<usize, S::Error>,
    S: Stream,
{
    type Output = ();

    type Handler = Self;

    fn wrap<C, W>(self, ctx: &C, wrapper: W) -> Self::Output
    where
        C: AsIoContext,
        W: FnOnce(&IoContext, Self::Handler),
    {
        wrapper(ctx.as_ctx(), self)
    }

    // fn wrap_timeout<C, W>(self, ctx: &C, _: Timeout, wrapper: W) -> Self::Output
    //     where C: Cancel,
    //           W: FnOnce(&IoContext, Self::Handler)
    // {
    //     wrapper(ctx.as_ctx(), self)
    // }
}

impl<F, S> Complete<usize, S::Error> for AsyncWriteAt<F, S>
where
    F: Complete<usize, S::Error>,
    S: Stream,
{
    fn success(mut self, this: &mut ThreadIoContext, len: usize) {
        let soc = unsafe { &*self.soc };
        let sbuf = unsafe { &mut *self.sbuf };
        sbuf.consume(len);
        self.left -= len;
        if self.left == 0 {
            self.handler.success(this, self.len)
        } else {
            let buf = &sbuf.as_bytes()[..self.left];
            soc.async_write_some(buf, self)
        }
    }

    fn failure(self, this: &mut ThreadIoContext, err: S::Error) {
        self.handler.failure(this, err)
    }
}

pub trait Stream: Cancel + Sized + Send + 'static {
    type Error: From<io::Error> + Send;

    fn async_read_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, Self::Error>;

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, Self::Error>;

    fn async_read_to_end<F>(&self, sbuf: &mut StreamBuf, handler: F) -> F::Output
    where
        F: Handler<usize, Self::Error>,
    {
        handler.wrap(self, move |ctx, handler| {
            let sbuf_ptr = sbuf as *mut _;
            match sbuf.prepare(4096) {
                Ok(buf) => {
                    self.async_read_some(
                        buf,
                        AsyncReadToEnd {
                            soc: self,
                            sbuf: sbuf_ptr,
                            len: 0,
                            handler: handler,
                        },
                    )
                }
                Err(err) => self.as_ctx().do_dispatch(Failure::new(err, handler)),
            }
        })
    }

    fn async_read_until<M, F>(&self, sbuf: &mut StreamBuf, cond: M, handler: F) -> F::Output
    where
        M: MatchCond,
        F: Handler<usize, Self::Error>,
    {
        handler.wrap(self, move |ctx, handler| {
            let sbuf_ptr = sbuf as *mut _;
            match sbuf.prepare(4096) {
                Ok(buf) => {
                    self.async_read_some(
                        buf,
                        AsyncReadUntil {
                            soc: self,
                            sbuf: sbuf_ptr,
                            cur: 0,
                            cond: cond,
                            handler: handler,
                        },
                    )
                }
                Err(err) => self.as_ctx().do_dispatch(Failure::new(err, handler)),
            }
        })
    }

    fn async_write_all<M, F>(&self, sbuf: &mut StreamBuf, handler: F) -> F::Output
    where
        M: MatchCond,
        F: Handler<usize, Self::Error>,
    {
        handler.wrap(self, move |ctx, handler| {
            let sbuf_ptr = sbuf as *mut _;
            let buf = sbuf.as_bytes();
            let len = buf.len();
            self.async_write_some(
                buf,
                AsyncWriteAt {
                    soc: self,
                    sbuf: sbuf_ptr,
                    len: 0,
                    left: len,
                    handler: handler,
                },
            )
        })
    }

    fn async_write_until<M, F>(&self, sbuf: &mut StreamBuf, mut cond: M, handler: F) -> F::Output
    where
        M: MatchCond,
        F: Handler<usize, Self::Error>,
    {
        handler.wrap(self, move |ctx, handler| {
            let sbuf_ptr = sbuf as *mut _;
            let buf = sbuf.as_bytes();
            let len = cond.match_cond(buf).unwrap_or(buf.len());
            self.async_write_some(
                buf,
                AsyncWriteAt {
                    soc: self,
                    sbuf: sbuf_ptr,
                    len: 0,
                    left: len,
                    handler: handler,
                },
            )
        })
    }
}

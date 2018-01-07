use prelude::{Protocol};
use core::{ThreadIoContext, Task};
use async::{Handler, Complete, NoYield};
use streams::{Stream, StreamBuf, MatchCond};

use std::io;
use std::marker::PhantomData;

pub struct ErrorHandler<F, R, E>(F, E, PhantomData<R>);

impl<F ,R ,E> ErrorHandler<F, R, E> {
    pub fn new(handler: F, err: E) -> Self {
        ErrorHandler(handler, err, PhantomData)
    }
}

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


pub struct AsyncReadToEnd<S, F> {
    soc: *const S,
    sbuf: *mut StreamBuf,
    len: usize,
    handler: F,
}

impl<S, F> AsyncReadToEnd<S, F> {
    pub fn new(soc: &S, sbuf: *mut StreamBuf, handler: F) -> Self {
        AsyncReadToEnd {
            soc: soc,
            sbuf: sbuf,
            len: 0,
            handler: handler,
        }
    }
}

unsafe impl<S, F> Send for AsyncReadToEnd<S, F> {}

impl<S, F> Handler<usize, io::Error> for AsyncReadToEnd<S, F>
    where S: Stream,
          F: Complete<usize, io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<S, F> Complete<usize, io::Error> for AsyncReadToEnd<S, F>
    where S: Stream,
          F: Complete<usize, io::Error>,
{

    fn success(mut self, this: &mut ThreadIoContext, len: usize) {
        self.len += len;
        let soc = unsafe { &*self.soc };
        let sbuf = unsafe { &mut *self.sbuf };
        match sbuf.prepare(4096) {
            Ok(buf) =>
                soc.async_read_some(buf, self),
            Err(err) =>
                self.handler.failure(this, err),
        }
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        if self.len > 0 {
            self.handler.success(this, self.len)
        } else {
            self.handler.failure(this, err)
        }
    }
}


pub struct AsyncReadUntil<S, M, F> {
    soc: *const S,
    sbuf: *mut StreamBuf,
    cur: usize,
    cond: M,
    handler: F,
}

impl<S, M, F> AsyncReadUntil<S, M, F> {
    pub fn new(soc: &S, sbuf: *mut StreamBuf, cond: M, handler: F) -> Self {
        AsyncReadUntil {
            soc: soc,
            sbuf: sbuf,
            cur: 0,
            cond: cond,
            handler: handler,
        }
    }
}

unsafe impl<S, M, F> Send for AsyncReadUntil<S, M, F> {}

impl<S, M, F> Handler<usize, io::Error> for AsyncReadUntil<S, M, F>
    where S: Stream,
          M: MatchCond,
          F: Complete<usize, io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<S, M, F> Complete<usize, io::Error> for AsyncReadUntil<S, M, F>
    where S: Stream,
          M: MatchCond,
          F: Complete<usize, io::Error>,
{
    fn success(mut self, this: &mut ThreadIoContext, len: usize) {
        let soc = unsafe { &*self.soc };
        let sbuf = unsafe { &mut *self.sbuf };
        let cur = self.cur;
        sbuf.commit(len);
        match self.cond.match_cond(&sbuf.as_bytes()[cur..]) {
            Ok(len) =>
                self.handler.success(this, cur + len),
            Err(len) =>
                match sbuf.prepare(4096) {
                    Ok(buf) => {
                        self.cur += len;
                        soc.async_read_some(buf, self)
                    },
                    Err(err) =>
                        self.failure(this, err),
                },
        }
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        self.handler.failure(this, err)
    }
}


pub struct AsyncWriteAt<S, F> {
    soc: *const S,
    sbuf: *mut StreamBuf,
    len: usize,
    cur: usize,
    handler: F,
}

impl<S, F> AsyncWriteAt<S, F> {
    pub fn new(soc: &S, sbuf: *mut StreamBuf, len: usize, handler: F) -> Self {
        AsyncWriteAt {
            soc: soc as *const _,
            sbuf: sbuf,
            len: len,
            cur: len,
            handler: handler,
        }
    }
}

unsafe impl<S, F> Send for AsyncWriteAt<S, F> {}

impl<S, F> Handler<usize, io::Error> for AsyncWriteAt<S, F>
    where S: Stream,
          F: Complete<usize, io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<S, F> Complete<usize, io::Error> for AsyncWriteAt<S, F>
    where S: Stream,
          F: Complete<usize, io::Error>,
{
    fn success(mut self, this: &mut ThreadIoContext, len: usize) {
        let soc = unsafe { &*self.soc };
        let sbuf = unsafe { &mut *self.sbuf };
        sbuf.consume(len);
        self.cur -= len;
        if self.cur == 0 {
            self.handler.success(this, self.len)
        } else {
            let buf = &sbuf.as_bytes()[..self.cur];
            soc.async_write_some(buf, self)
        }
    }

    fn failure(self, this: &mut ThreadIoContext, err: io::Error) {
        self.handler.failure(this, err)
    }
}

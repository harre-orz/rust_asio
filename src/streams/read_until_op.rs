use prelude::*;
use core::ThreadIoContext;
use streams::{Stream, StreamBuf, MatchCond};
use async::{Handler, Complete, NoYield};

use std::io;
use std::marker::PhantomData;


pub struct AsyncReadToEnd<P, S, F> {
    soc: *const S,
    sbuf: *mut StreamBuf,
    len: usize,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, F> AsyncReadToEnd<P, S, F> {
    pub fn new(soc: &S, sbuf: *mut StreamBuf, handler: F) -> Self {
        AsyncReadToEnd {
            soc: soc,
            sbuf: sbuf,
            len: 0,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<P, S, F> Send for AsyncReadToEnd<P, S, F> {}

impl<P, S, F> Handler<usize, io::Error> for AsyncReadToEnd<P, S, F>
    where P: Protocol,
          S: Stream<P>,
          F: Complete<usize, io::Error>,
{
    type Output = ();

    type Perform = Self;

    type Yield = NoYield;

    fn channel(self) -> (Self::Perform, Self::Yield) {
        (self, NoYield)
    }
}

impl<P, S, F> Complete<usize, io::Error> for AsyncReadToEnd<P, S, F>
    where P: Protocol,
          S: Stream<P>,
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


pub struct AsyncReadUntil<P, S, M, F> {
    soc: *const S,
    sbuf: *mut StreamBuf,
    cur: usize,
    cond: M,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, M, F> AsyncReadUntil<P, S, M, F> {
    pub fn new(soc: &S, sbuf: *mut StreamBuf, cond: M, handler: F) -> Self {
        AsyncReadUntil {
            soc: soc,
            sbuf: sbuf,
            cur: 0,
            cond: cond,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<P, S, M, F> Send for AsyncReadUntil<P, S, M, F> {}

impl<P, S, M, F> Handler<usize, io::Error> for AsyncReadUntil<P, S, M, F>
    where P: Protocol,
          S: Stream<P>,
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

impl<P, S, M, F> Complete<usize, io::Error> for AsyncReadUntil<P, S, M, F>
    where P: Protocol,
          S: Stream<P>,
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

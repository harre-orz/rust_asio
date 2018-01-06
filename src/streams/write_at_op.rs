use prelude::*;
use core::ThreadIoContext;
use streams::{Stream, StreamBuf, MatchCond};
use async::{Handler, Complete, NoYield};

use std::io;
use std::marker::PhantomData;


pub struct AsyncWriteAt<P, S, F> {
    soc: *const S,
    sbuf: *mut StreamBuf,
    len: usize,
    cur: usize,
    handler: F,
    _marker: PhantomData<P>,
}

impl<P, S, F> AsyncWriteAt<P, S, F> {
    pub fn new(soc: &S, sbuf: *mut StreamBuf, len: usize, handler: F) -> Self {
        AsyncWriteAt {
            soc: soc as *const _,
            sbuf: sbuf,
            len: len,
            cur: len,
            handler: handler,
            _marker: PhantomData,
        }
    }
}

unsafe impl<P, S, F> Send for AsyncWriteAt<P, S, F> {}

impl<P, S, F> Handler<usize, io::Error> for AsyncWriteAt<P, S, F>
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

impl<P, S, F> Complete<usize, io::Error> for AsyncWriteAt<P, S, F>
    where P: Protocol,
          S: Stream<P>,
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

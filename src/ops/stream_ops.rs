use core::ThreadIoContext;
use ops::{Complete, Handler, NoYield};
use streams::{MatchCond, Stream, StreamBuf};

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

impl<S, F> Complete<usize, S::Error> for AsyncReadToEnd<S, F>
where
    S: Stream,
    F: Complete<usize, S::Error>,
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

impl<S, F> Handler<usize, S::Error> for AsyncReadToEnd<S, F>
where
    S: Stream,
    F: Complete<usize, S::Error>,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
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

impl<S, M, F> Complete<usize, S::Error> for AsyncReadUntil<S, M, F>
where
    S: Stream,
    M: MatchCond,
    F: Complete<usize, S::Error>,
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

impl<S, M, F> Handler<usize, S::Error> for AsyncReadUntil<S, M, F>
where
    S: Stream,
    M: MatchCond,
    F: Complete<usize, S::Error>,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
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

impl<S, F> Complete<usize, S::Error> for AsyncWriteAt<S, F>
where
    S: Stream,
    F: Complete<usize, S::Error>,
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

    fn failure(self, this: &mut ThreadIoContext, err: S::Error) {
        self.handler.failure(this, err)
    }
}

impl<S, F> Handler<usize, S::Error> for AsyncWriteAt<S, F>
where
    S: Stream,
    F: Complete<usize, S::Error>,
{
    type Output = ();

    type Caller = Self;

    type Callee = NoYield;

    fn channel(self) -> (Self::Caller, Self::Callee) {
        (self, NoYield)
    }
}

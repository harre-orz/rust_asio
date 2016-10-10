use std::io;
use unsafe_cell::{UnsafeRefCell};
use io_service::{IoObject, IoService, Handler, AsyncResult};
use buffer::{StreamBuf, MatchCondition};

pub trait Stream : IoObject + Send + 'static {
    fn async_read_some<F>(&self, buf: &mut [u8], handler: F) -> F::Output
        where F: Handler<usize>;

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
        where F: Handler<usize>;

    fn read_some(&self, buf: &mut [u8]) -> io::Result<usize>;

    fn write_some(&self, buf: &[u8]) -> io::Result<usize>;
}

pub fn read_until<S, M>(s: &S, sbuf: &mut StreamBuf, mut cond: M) -> io::Result<usize>
    where S: Stream,
          M: MatchCondition,
{
    let mut cur = 0;
    loop {
        match cond.match_cond(&sbuf.as_slice()[cur..]) {
            Ok(len) => return Ok(cur + len),
            Err(len) => {
                cur += len;
                let len = try!(s.read_some(try!(sbuf.prepare(4096))));
                sbuf.commit(len);
            },
        }
    }
}

struct ReadUntilHandler<S, M, F> {
    s: UnsafeRefCell<S>,
    sbuf: UnsafeRefCell<StreamBuf>,
    cond: M,
    handler: F,
    cur: usize,
}

impl<S, M, F> Handler<usize> for ReadUntilHandler<S, M, F>
    where S: Stream,
          M: MatchCondition,
          F: Handler<usize>,
{
    type Output = F::Output;

    type AsyncResult = F::AsyncResult;

    fn async_result(&self) -> Self::AsyncResult {
        self.handler.async_result()
    }

    fn callback(self, io: &IoService, res: io::Result<usize>) {
        let ReadUntilHandler { s, mut sbuf, cond, handler, cur } = self;
        let s = unsafe { s.as_ref() };
        match res {
            Ok(len) => {
                let sbuf = unsafe { sbuf.as_mut() };
                sbuf.commit(len);
                async_read_until_impl(s, sbuf, cond, handler, cur);
            },
            Err(err) => handler.callback(io, Err(err)),
        }
    }
}

fn async_read_until_impl<S, M, F>(s: &S, sbuf: &mut StreamBuf, mut cond: M, handler: F, mut cur: usize) -> F::Output
    where S: Stream,
          M: MatchCondition,
          F: Handler<usize>,
{
    let io = s.io_service();
    let out = handler.async_result();
    match cond.match_cond(&sbuf.as_slice()[cur..]) {
        Ok(len) => handler.callback(io, Ok(cur + len)),
        Err(len) => {
            cur += len;
            let sbuf_ptr = UnsafeRefCell::new(sbuf);
            match sbuf.prepare(4096) {
                Ok(buf) => {
                    let handler = ReadUntilHandler {
                        s: UnsafeRefCell::new(s),
                        sbuf: sbuf_ptr,
                        cond: cond,
                        handler: handler,
                        cur: cur,
                    };
                    s.async_read_some(buf, handler);
                },
                Err(err) => handler.callback(io, Err(err)),
            }
        }
    }
    out.get(io)
}

pub fn async_read_until<S, M, F>(s: &S, sbuf: &mut StreamBuf, cond: M, handler: F) -> F::Output
    where S: Stream,
          M: MatchCondition,
          F: Handler<usize>,
{
    async_read_until_impl(s, sbuf, cond, handler, 0)
}

pub fn write_until<S, M>(s: &S, sbuf: &mut StreamBuf, mut cond: M) -> io::Result<usize>
    where S: Stream,
          M: MatchCondition,
{
    let len = {
        let len = match cond.match_cond(sbuf.as_slice()) {
            Ok(len) => len,
            Err(len) => len,
        };
        try!(s.write_some(&sbuf.as_slice()[..len]))
    };
    sbuf.consume(len);
    Ok(len)
}

struct WriteUntilHandler<S, F> {
    s: UnsafeRefCell<S>,
    sbuf: UnsafeRefCell<StreamBuf>,
    handler: F,
    total: usize,
    cur: usize,
}

impl<S, F> Handler<usize> for WriteUntilHandler<S, F>
    where S: Stream,
          F: Handler<usize>,
{
    type Output = F::Output;

    type AsyncResult = F::AsyncResult;

    fn async_result(&self) -> Self::AsyncResult {
        self.handler.async_result()
    }

    fn callback(self, io: &IoService, res: io::Result<usize>) {
        let WriteUntilHandler { s, mut sbuf, handler, total, mut cur } = self;
        let s = unsafe { s.as_ref() };
        match res {
            Ok(len) => {
                let sbuf = unsafe { sbuf.as_mut() };
                sbuf.consume(len);
                cur -= len;
                if cur == 0 {
                    handler.callback(io, Ok(total))
                } else {
                    async_write_until_impl(s, sbuf, len, handler, cur);
                }
            },
            Err(err) => handler.callback(io, Err(err)),
        }
    }
}

fn async_write_until_impl<S, F>(s: &S, sbuf: &mut StreamBuf, total: usize, handler: F, cur: usize) -> F::Output
    where S: Stream,
          F: Handler<usize>,
{
    let handler = WriteUntilHandler {
        s: UnsafeRefCell::new(s),
        sbuf: UnsafeRefCell::new(sbuf),
        handler: handler,
        total: total,
        cur: cur,
    };
    s.async_write_some(&sbuf.as_slice()[..cur], handler)
}

pub fn async_write_until<S, M, F>(s: &S, sbuf: &mut StreamBuf, mut cond: M, handler: F) -> F::Output
    where S: Stream,
          M: MatchCondition,
          F: Handler<usize>,
{
    let total = match cond.match_cond(sbuf.as_slice()) {
        Ok(len) => len,
        Err(len) => len,
    };
    async_write_until_impl(s, sbuf, total, handler, total)
}

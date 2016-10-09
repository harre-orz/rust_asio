use std::io;
use std::cmp;
use unsafe_cell::{UnsafeRefCell};
use {IoObject, IoService};
use async_result::{Handler, AsyncResult};

fn length_error() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Length error")
}

pub struct StreamBuf {
    buf: Vec<u8>,
    cur: usize,
    max: usize,
}

impl StreamBuf {
    pub fn new(max: usize) -> StreamBuf {
        StreamBuf {
            buf: Vec::new(),
            cur: 0,
            max: max,
        }
    }

    pub fn max_len(&self) -> usize {
        self.max
    }

    pub fn len(&self) -> usize {
        self.cur
    }

    pub fn prepare(&mut self, len: usize) -> io::Result<&mut [u8]> {
        if self.cur < self.max {
            let len = cmp::min(self.cur + len, self.max);
            self.buf.reserve(len);
            unsafe { self.buf.set_len(len); }
            Ok(&mut self.buf[self.cur..])
        } else {
            Err(length_error())
        }
    }

    pub fn prepare_exact(&mut self, mut len: usize) -> io::Result<&mut [u8]> {
        len += self.cur;
        if len <= self.max {
            self.buf.reserve(len);
            unsafe { self.buf.set_len(len); }
            Ok(&mut self.buf[self.cur..])
        } else {
            Err(length_error())
        }
    }

    pub fn commit(&mut self, len: usize) {
        self.cur = cmp::min(self.cur + len, self.buf.len());
    }

    pub fn consume(&mut self, mut len: usize) {
        if len > self.cur { len = self.cur; }
        self.buf.drain(..len);
        self.cur -= len;
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf[..self.cur]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.buf[..self.cur]
    }
}

impl io::Read for StreamBuf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = cmp::min(self.cur, buf.len());
        buf[..len].clone_from_slice(&self.as_slice());
        self.consume(len);
        Ok(len)
    }
}

impl io::Write for StreamBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len();
        try!(self.prepare_exact(len)).clone_from_slice(buf);
        self.commit(len);
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub trait MatchCondition : Send + 'static {
    fn is_match(&mut self, buf: &[u8]) -> Result<usize, usize>;
}

impl MatchCondition for usize {
    fn is_match(&mut self, buf: &[u8]) -> Result<usize, usize> {
        if buf.len() >= *self {
            Ok(*self)
        } else {
            *self -= buf.len();
            Err(buf.len())
        }
    }
}

impl MatchCondition for u8 {
    fn is_match(&mut self, buf: &[u8]) -> Result<usize, usize> {
        if let Some(len) = buf.iter().position(|&x| x == *self) {
            Ok(len+1)
        } else {
            Err(buf.len())
        }
    }
}

impl MatchCondition for &'static [u8] {
    fn is_match(&mut self, buf: &[u8]) -> Result<usize, usize> {
        let mut cur = 0;
        if !self.is_empty() {
            let head = self[0];
            let tail = &self[1..];
            let mut it = buf.iter();
            while let Some(mut len) = it.position(|&x| x == head) {
                len += cur + 1;
                let buf = &buf[len..];
                if buf.len() < tail.len() {
                    return Err(len - 1);
                } else if buf.starts_with(tail) {
                    return Ok(len + tail.len());
                }
                cur = len;
                it = buf.iter();
            }
            cur = buf.len();
        }
        Err(cur)
    }
}

impl MatchCondition for char {
    fn is_match(&mut self, buf: &[u8]) -> Result<usize, usize> {
        (*self as u8).is_match(buf)
    }
}

impl MatchCondition for &'static str {
    fn is_match(&mut self, buf: &[u8]) -> Result<usize, usize> {
        self.as_bytes().is_match(buf)
    }
}

pub trait Stream : IoObject + Send + 'static {
    fn async_read_some<F: Handler<usize>>(&self, buf: &mut [u8], handler: F) -> F::Output;

    fn async_write_some<F: Handler<usize>>(&self, buf: &[u8], handler: F) -> F::Output;

    fn read_some(&self, buf: &mut [u8]) -> io::Result<usize>;

    fn write_some(&self, buf: &[u8]) -> io::Result<usize>;
}

pub fn read_until<S: Stream, C: MatchCondition>(s: &S, sbuf: &mut StreamBuf, mut cond: C) -> io::Result<usize> {
    let mut cur = 0;
    loop {
        match cond.is_match(&sbuf.as_slice()[cur..]) {
            Ok(len) => return Ok(cur + len),
            Err(len) => {
                cur += len;
                let len = try!(s.read_some(try!(sbuf.prepare(4096))));
                sbuf.commit(len);
            },
        }
    }
}

struct ReadUntilHandler<S, C, F> {
    s: UnsafeRefCell<S>,
    sbuf: UnsafeRefCell<StreamBuf>,
    cond: C,
    handler: F,
    cur: usize,
}

impl<S, C, F> Handler<usize> for ReadUntilHandler<S, C, F>
    where S: Stream,
          C: MatchCondition,
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

fn async_read_until_impl<S: Stream, C: MatchCondition, F: Handler<usize>>(s: &S, sbuf: &mut StreamBuf, mut cond: C, handler: F, mut cur: usize) -> F::Output {
    let io = s.io_service();
    let out = handler.async_result();
    match cond.is_match(&sbuf.as_slice()[cur..]) {
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
    out.result(io)
}

pub fn async_read_until<S: Stream, C: MatchCondition, F: Handler<usize>>(s: &S, sbuf: &mut StreamBuf, cond: C, handler: F) -> F::Output {
    async_read_until_impl(s, sbuf, cond, handler, 0)
}

pub fn write_until<S: Stream, C: MatchCondition>(s: &S, sbuf: &mut StreamBuf, mut cond: C) -> io::Result<usize> {
    let len = {
        let len = match cond.is_match(sbuf.as_slice()) {
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

fn async_write_until_impl<S: Stream, F: Handler<usize>>(s: &S, sbuf: &mut StreamBuf, total: usize, handler: F, cur: usize) -> F::Output {
    let handler = WriteUntilHandler {
        s: UnsafeRefCell::new(s),
        sbuf: UnsafeRefCell::new(sbuf),
        handler: handler,
        total: total,
        cur: cur,
    };
    s.async_write_some(&sbuf.as_slice()[..cur], handler)
}

pub fn async_write_until<S: Stream, C: MatchCondition, F: Handler<usize>>(s: &S, sbuf: &mut StreamBuf, mut cond: C, handler: F) -> F::Output {
    let total = match cond.is_match(sbuf.as_slice()) {
        Ok(len) => len,
        Err(len) => len,
    };
    async_write_until_impl(s, sbuf, total, handler, total)
}

#[test]
fn test_streambuf() {
    let sbuf = StreamBuf::new(100);
    assert_eq!(sbuf.len(), 0);
    assert_eq!(sbuf.max_len(), 100);
}

#[test]
fn test_streambuf_prepare() {
    let mut sbuf = StreamBuf::new(100);
    assert_eq!(sbuf.prepare(70).unwrap().len(), 70);
    sbuf.commit(70);
    assert_eq!(sbuf.len(), 70);
    assert_eq!(sbuf.prepare(70).unwrap().len(), 30);
    sbuf.commit(70);
    assert_eq!(sbuf.len(), 100);
}

#[test]
fn test_streambuf_prepare_exact() {
    let mut sbuf = StreamBuf::new(100);
    assert_eq!(sbuf.prepare_exact(70).unwrap().len(), 70);
    sbuf.commit(70);
    assert_eq!(sbuf.len(), 70);
    assert!(sbuf.prepare_exact(70).is_err());
    sbuf.commit(70);
    assert_eq!(sbuf.len(), 70);
}

#[test]
fn test_streambuf_consume() {
    let mut sbuf = StreamBuf::new(100);
    assert_eq!(sbuf.prepare_exact(1).unwrap().len(), 1);
    assert_eq!(sbuf.prepare_exact(100).unwrap().len(), 100);
    sbuf.commit(1);
    assert_eq!(sbuf.len(), 1);
    assert!(sbuf.prepare_exact(100).is_err());
    sbuf.consume(1);
    assert_eq!(sbuf.len(), 0);
    assert!(sbuf.prepare_exact(100).is_ok());
}

#[test]
fn test_match_cond() {
    assert!((5 as usize).is_match("hello".as_bytes()) == Ok(5));
    assert!((5 as usize).is_match("hello world".as_bytes()) == Ok(5));
    assert!((10 as usize).is_match("hello".as_bytes()) == Err(5));
    assert!('l'.is_match("hello".as_bytes()) == Ok(3));
    assert!('w'.is_match("hello".as_bytes()) == Err(5));
    assert!("lo".is_match("hello world".as_bytes()) == Ok(5));
    assert!("world!".is_match("hello world".as_bytes()) == Err(6));
    assert!("".is_match("hello".as_bytes()) == Err(0));
    assert!("l".is_match("hello".as_bytes()) == Ok(3));
}

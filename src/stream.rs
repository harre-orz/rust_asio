use std::io;
use std::cmp;
use {IoObject, IoService, Handler};
use backbone::ops::{UnsafeRefCell};

fn length_error() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "E2BIG")
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
            // TODO: メモリ確保に失敗したときも Err にしたい
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
            // TODO: メモリ確保に失敗したときも Err にしたい
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
    fn async_read_some<F: Handler<Self, usize>>(&self, buf: &mut [u8], handler: F);

    fn async_write_some<F: Handler<Self, usize>>(&self, buf: &[u8], handler: F);

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

struct ReadUntilHandler<C, F> {
    sbuf: *mut StreamBuf,
    cond: C,
    handler: F,
    cur: usize,
}

unsafe impl<C, F> Send for ReadUntilHandler<C, F> {}

impl<S, C, F> Handler<S, usize> for ReadUntilHandler<C, F>
    where S: Stream,
          C: MatchCondition,
          F: Handler<S, usize>,
{
    fn callback(self, io: &IoService, s: &S, res: io::Result<usize>) {
        let ReadUntilHandler { sbuf, cond, handler, cur } = self;
        match res {
            Ok(len) => {
                let sbuf = unsafe { &mut *sbuf };
                sbuf.commit(len);
                async_read_until_impl(s, sbuf, cond, handler, cur);
            },
            Err(err) =>
                handler.callback(io, s, Err(err)),
        }
    }
}

fn async_read_until_impl<S: Stream, C: MatchCondition, F: Handler<S, usize>>(s: &S, sbuf: &mut StreamBuf, mut cond: C, handler: F, mut cur: usize) {
    let io = s.io_service();
    match cond.is_match(&sbuf.as_slice()[cur..]) {
        Ok(len) => {
            if cur > 0 {
                handler.callback(io, s, Ok(cur + len));
            } else {
                let s_ptr = UnsafeRefCell::new(s);
                io.post(move |io| handler.callback(io, unsafe { s_ptr.as_ref() }, Ok(cur + len)));
            }
        },
        Err(len) => {
            cur += len;
            let sbuf_ptr = sbuf as *mut StreamBuf;
            match sbuf.prepare(4096) {
                Ok(buf) => {
                    let handler = ReadUntilHandler {
                        sbuf: sbuf_ptr,
                        cond: cond,
                        handler: handler,
                        cur: cur,
                    };
                    s.async_read_some(buf, handler);
                },
                Err(err) => {
                    if cur > len {
                        handler.callback(io, s, Ok(cur + len));
                    } else {
                        let s_ptr = UnsafeRefCell::new(s);
                        io.post(move |io| handler.callback(io, unsafe { s_ptr.as_ref() }, Err(err)));
                    }
                },
            }
        }
    }
}

pub fn async_read_until<S: Stream, C: MatchCondition, F: Handler<S, usize>>(s: &S, sbuf: &mut StreamBuf, cond: C, handler: F) {
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

struct WriteUntilHandler<F> {
    sbuf: *mut StreamBuf,
    handler: F,
    total: usize,
    cur: usize,
}

unsafe impl<F> Send for WriteUntilHandler<F> {}

impl<S, F> Handler<S, usize> for WriteUntilHandler<F>
    where S: Stream,
          F: Handler<S, usize>,
{
    fn callback(self, io: &IoService, s: &S, res: io::Result<usize>) {
        let WriteUntilHandler { sbuf, handler, total, mut cur } = self;
        match res {
            Ok(len) => {
                let sbuf = unsafe { &mut *sbuf };
                sbuf.consume(len);
                cur -= len;
                if cur == 0 {
                    handler.callback(io, s, Ok(total))
                } else {
                    async_write_until_impl(s, sbuf, len, handler, cur);
                }
            },
            Err(err) =>
                handler.callback(io, s, Err(err)),
        }
    }
}

fn async_write_until_impl<S: Stream, F: Handler<S, usize>>(s: &S, sbuf: &mut StreamBuf, total: usize, handler: F, cur: usize) {
    let handler = WriteUntilHandler {
        sbuf: sbuf as *mut StreamBuf,
        handler: handler,
        total: total,
        cur: cur,
    };
    s.async_write_some(&sbuf.as_slice()[..cur], handler);
}

pub fn async_write_until<S: Stream, C: MatchCondition, F: Handler<S, usize>>(s: &S, sbuf: &mut StreamBuf, mut cond: C, handler: F) {
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

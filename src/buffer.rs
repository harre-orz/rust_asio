use std::io;
use std::cmp;

fn length_error() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "Length error")
}

#[derive(Debug)]
pub struct StreamBuf {
    buf: Vec<u8>,
    cur: usize,
    max: usize,
}

impl StreamBuf {
    pub fn new() -> StreamBuf {
        Self::with_max_len(usize::max_value())
    }

    pub fn with_max_len(max: usize) -> StreamBuf {
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

impl Default for StreamBuf {
    fn default() -> Self {
        StreamBuf::new()
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
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize>;
}

impl MatchCondition for usize {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        if buf.len() >= *self {
            Ok(*self)
        } else {
            *self -= buf.len();
            Err(buf.len())
        }
    }
}

impl MatchCondition for u8 {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        if let Some(len) = buf.iter().position(|&x| x == *self) {
            Ok(len+1)
        } else {
            Err(buf.len())
        }
    }
}

impl MatchCondition for &'static [u8] {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
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
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        (*self as u8).match_cond(buf)
    }
}

impl MatchCondition for &'static str {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        self.as_bytes().match_cond(buf)
    }
}

#[test]
fn test_streambuf() {
    let sbuf = StreamBuf::with_max_len(100);
    assert_eq!(sbuf.len(), 0);
    assert_eq!(sbuf.max_len(), 100);
}

#[test]
fn test_streambuf_prepare() {
    let mut sbuf = StreamBuf::with_max_len(100);
    assert_eq!(sbuf.prepare(70).unwrap().len(), 70);
    sbuf.commit(70);
    assert_eq!(sbuf.len(), 70);
    assert_eq!(sbuf.prepare(70).unwrap().len(), 30);
    sbuf.commit(70);
    assert_eq!(sbuf.len(), 100);
}

#[test]
fn test_streambuf_prepare_exact() {
    let mut sbuf = StreamBuf::with_max_len(100);
    assert_eq!(sbuf.prepare_exact(70).unwrap().len(), 70);
    sbuf.commit(70);
    assert_eq!(sbuf.len(), 70);
    assert!(sbuf.prepare_exact(70).is_err());
    sbuf.commit(70);
    assert_eq!(sbuf.len(), 70);
}

#[test]
fn test_streambuf_consume() {
    let mut sbuf = StreamBuf::with_max_len(100);
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
    assert!((5 as usize).match_cond("hello".as_bytes()) == Ok(5));
    assert!((5 as usize).match_cond("hello world".as_bytes()) == Ok(5));
    assert!((10 as usize).match_cond("hello".as_bytes()) == Err(5));
    assert!('l'.match_cond("hello".as_bytes()) == Ok(3));
    assert!('w'.match_cond("hello".as_bytes()) == Err(5));
    assert!("lo".match_cond("hello world".as_bytes()) == Ok(5));
    assert!("world!".match_cond("hello world".as_bytes()) == Err(6));
    assert!("".match_cond("hello".as_bytes()) == Err(0));
    assert!("l".match_cond("hello".as_bytes()) == Ok(3));
}

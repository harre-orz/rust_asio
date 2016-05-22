use std::io;
use std::cmp;

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
        match self.cur + len {
            n if n <= self.max => {
                self.buf.reserve(n);
                unsafe { self.buf.set_len(n); }
                Ok(&mut self.buf[self.cur..])
            },
            _ => Err(io::Error::new(io::ErrorKind::Other, "E2BIG"))
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
        try!(self.prepare(len)).clone_from_slice(buf);
        self.commit(len);
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub trait MatchCondition {
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

impl<'a> MatchCondition for &'a [u8] {
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

impl<'a> MatchCondition for &'a str {
    fn is_match(&mut self, buf: &[u8]) -> Result<usize, usize> {
        self.as_bytes().is_match(buf)
    }
}

impl<'a> MatchCondition for String {
    fn is_match(&mut self, buf: &[u8]) -> Result<usize, usize> {
        self.as_bytes().is_match(buf)
    }
}

#[test]
fn test_streambuf() {
    let mut sbuf = StreamBuf::new(65536);
    assert!(sbuf.len() == 0);
    assert!(sbuf.max_len() == 65536);
    assert!(sbuf.prepare(1).is_ok());
    assert!(sbuf.prepare(65536).is_ok());
    assert!(sbuf.prepare(65537).is_err());
    sbuf.commit(1);
    assert!(sbuf.len() == 1);
    assert!(sbuf.prepare(65536).is_err());
    sbuf.consume(1);
    assert!(sbuf.len() == 0);
    assert!(sbuf.prepare(65536).is_ok());
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

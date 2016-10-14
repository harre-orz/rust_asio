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
        Self::with_capacity(usize::max_value())
    }

    pub fn with_capacity(max: usize) -> StreamBuf {
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

#[test]
fn test_streambuf() {
    let sbuf = StreamBuf::with_capacity(100);
    assert_eq!(sbuf.len(), 0);
    assert_eq!(sbuf.max_len(), 100);
}

#[test]
fn test_streambuf_prepare() {
    let mut sbuf = StreamBuf::with_capacity(100);
    assert_eq!(sbuf.prepare(70).unwrap().len(), 70);
    sbuf.commit(70);
    assert_eq!(sbuf.len(), 70);
    assert_eq!(sbuf.prepare(70).unwrap().len(), 30);
    sbuf.commit(70);
    assert_eq!(sbuf.len(), 100);
}

#[test]
fn test_streambuf_prepare_exact() {
    let mut sbuf = StreamBuf::with_capacity(100);
    assert_eq!(sbuf.prepare_exact(70).unwrap().len(), 70);
    sbuf.commit(70);
    assert_eq!(sbuf.len(), 70);
    assert!(sbuf.prepare_exact(70).is_err());
    sbuf.commit(70);
    assert_eq!(sbuf.len(), 70);
}

#[test]
fn test_streambuf_consume() {
    let mut sbuf = StreamBuf::with_capacity(100);
    assert_eq!(sbuf.prepare_exact(1).unwrap().len(), 1);
    assert_eq!(sbuf.prepare_exact(100).unwrap().len(), 100);
    sbuf.commit(1);
    assert_eq!(sbuf.len(), 1);
    assert!(sbuf.prepare_exact(100).is_err());
    sbuf.consume(1);
    assert_eq!(sbuf.len(), 0);
    assert!(sbuf.prepare_exact(100).is_ok());
}

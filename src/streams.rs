use ffi::NO_BUFFER_SPACE;
use core::AsIoContext;
use handler::{Handler, Yield};
use ops::{AsyncReadToEnd, AsyncReadUntil, AsyncWriteAt, Failure};

use std::io;
use std::cmp;
use std::ffi::CString;

/// Automatically resizing buffer.
#[derive(Clone, Debug)]
pub struct StreamBuf {
    buf: Vec<u8>,
    max: usize,
    cur: usize,
    beg: usize,
}

impl StreamBuf {
    /// Returns a new `StreamBuf`.
    ///
    /// Equivalent to `with_max_len(usize::max_len())`
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::new();
    /// ```
    pub fn new() -> StreamBuf {
        Self::with_max_len(usize::max_value())
    }

    /// Returns a new `StreamBuf` with the max length of the allocatable size.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::with_max_len(1024);
    /// ```
    pub fn with_max_len(max: usize) -> StreamBuf {
        StreamBuf {
            buf: Vec::new(),
            max: max,
            cur: 0,
            beg: 0,
        }
    }

    /// Returns an allocated size of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::new();
    /// assert_eq!(sbuf.capacity(), 0);
    /// ```
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Clears the buffer, removing all values.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::from(vec![1,2,3]);
    /// sbuf.clear();
    /// assert_eq!(sbuf.is_empty(), true);
    /// ```
    pub fn clear(&mut self) {
        self.buf.clear();
        self.cur = 0;
        self.beg = 0;
    }

    /// Remove characters from the input sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::from(vec![1,2,3]);
    /// assert_eq!(sbuf.len(), 3);
    /// sbuf.consume(3);
    /// assert_eq!(sbuf.len(), 0);
    /// ```
    pub fn consume(&mut self, len: usize) {
        if len >= self.len() {
            self.clear()
        } else {
            self.beg += len;
        }
    }

    /// Move characters from the output sequence to the input sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::new();
    /// let _ = sbuf.prepare(256);
    /// assert_eq!(sbuf.len(), 0);
    /// sbuf.commit(3);
    /// assert_eq!(sbuf.len(), 3);
    /// ```
    pub fn commit(&mut self, len: usize) {
        self.cur = cmp::min(self.cur + len, self.buf.len());
    }

    /// Returns `true` if the empty buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::StreamBuf;
    ///
    /// let sbuf = StreamBuf::new();
    /// assert!(sbuf.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Returns a length of the input sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::StreamBuf;
    ///
    /// let sbuf = StreamBuf::from(vec![1,2,3]);
    /// assert_eq!(sbuf.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.cur - self.beg
    }

    /// Returns a maximum length of the `StreamBuf`.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::StreamBuf;
    ///
    /// let sbuf = StreamBuf::new();
    /// assert_eq!(sbuf.max_len(), usize::max_value());
    /// ```
    pub fn max_len(&self) -> usize {
        self.max
    }

    /// Returns a `&mut [u8]` that represents a output sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::with_max_len(8);
    /// assert_eq!(sbuf.prepare(5).unwrap().len(), 5);
    /// sbuf.commit(5);
    /// assert_eq!(sbuf.prepare(5).unwrap().len(), 3);
    /// ```
    pub fn prepare(&mut self, len: usize) -> io::Result<&mut [u8]> {
        if self.cur + len <= self.max {
            self.buf.reserve(self.cur + len);
            unsafe { self.buf.set_len(self.cur + len) };
            Ok(&mut self.buf[self.cur..])
        } else if self.beg >= len {
            self.buf.drain(..self.beg);
            self.cur -= self.beg;
            unsafe { self.buf.set_len(len) };
            Ok(&mut self.buf[self.cur..])
        } else if self.len() < self.max {
            self.buf.drain(..self.beg);
            self.buf.reserve(self.max);
            self.cur -= self.beg;
            unsafe { self.buf.set_len(self.max) };
            Ok(&mut self.buf[self.cur..])
        } else {
            Err(NO_BUFFER_SPACE.into())
        }
    }

    /// Returns a `&mut [u8]` that represents a output sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::with_max_len(8);
    /// assert_eq!(sbuf.prepare_exact(5).unwrap().len(), 5);
    /// sbuf.commit(5);
    /// assert!(sbuf.prepare_exact(5).is_err());
    /// ```
    pub fn prepare_exact(&mut self, len: usize) -> io::Result<&mut [u8]> {
        if self.cur + len <= self.max {
            self.buf.reserve(self.cur + len);
            unsafe { self.buf.set_len(self.cur + len) };
            Ok(&mut self.buf[self.cur..])
        } else if self.beg >= len {
            self.buf.drain(..self.beg);
            self.cur -= self.beg;
            unsafe { self.buf.set_len(len) };
            Ok(&mut self.buf[self.cur..])
        } else {
            Err(NO_BUFFER_SPACE.into())
        }
    }

    /// Returns a `&[u8]` that represents the input sequence.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[self.beg..self.cur]
    }

    /// Returns a `&mut [u8]` that represents the input sequence.
    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        &mut self.buf[self.beg..self.cur]
    }
}

impl Default for StreamBuf {
    fn default() -> Self {
        StreamBuf::new()
    }
}

impl From<Vec<u8>> for StreamBuf {
    fn from(buf: Vec<u8>) -> Self {
        let len = buf.len();
        StreamBuf {
            buf: buf,
            max: usize::max_value(),
            cur: len,
            beg: 0,
        }
    }
}

impl From<CString> for StreamBuf {
    fn from(buf: CString) -> Self {
        StreamBuf::from(Vec::from(buf))
    }
}

impl<'a> From<&'a [u8]> for StreamBuf {
    fn from(buf: &'a [u8]) -> Self {
        StreamBuf::from(Vec::from(buf))
    }
}

impl<'a> From<&'a str> for StreamBuf {
    fn from(buf: &'a str) -> Self {
        StreamBuf::from(Vec::from(buf))
    }
}

impl io::Read for StreamBuf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = cmp::min(self.len(), buf.len());
        if len > 0 {
            buf[..len].clone_from_slice(&self.buf[self.beg..self.beg + len]);
            self.consume(len);
            Ok(len)
        } else {
            Err(NO_BUFFER_SPACE.into())
        }
    }
}

impl io::Write for StreamBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = {
            let x = self.prepare(buf.len())?;
            let len = x.len();
            x.clone_from_slice(&buf[..len]);
            len
        };
        self.commit(len);
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn match_cond_bytes_unchecked(buf: &[u8], head: u8, tail: &[u8]) -> Result<usize, usize> {
    let mut cur = 0;
    let mut it = buf.iter();
    while let Some(mut len) = it.position(|&ch| ch == head) {
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
    Err(buf.len())
}

pub trait MatchCond: Send + 'static {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize>;
}

impl MatchCond for &'static [u8] {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        if self.is_empty() {
            Err(0)
        } else {
            match_cond_bytes_unchecked(buf, self[0], &self[1..])
        }
    }
}

impl MatchCond for &'static str {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        self.as_bytes().match_cond(buf)
    }
}

impl MatchCond for char {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        use std::mem;
        let mut bytes: [u8; 4] = unsafe { mem::uninitialized() };
        let len = self.encode_utf8(&mut bytes).as_bytes().len();
        match_cond_bytes_unchecked(buf, bytes[0], &bytes[1..len])
    }
}

impl MatchCond for String {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        match_cond_bytes_unchecked(buf, self.as_bytes()[0], &self.as_bytes()[1..])
    }
}

impl MatchCond for usize {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        if buf.len() >= *self {
            Ok(*self)
        } else {
            *self -= buf.len();
            Err(buf.len())
        }
    }
}

pub trait Stream: AsIoContext + io::Read + io::Write + Sized + Send + 'static {
    fn async_read_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, io::Error>;

    fn async_write_some<F>(&self, buf: &[u8], handler: F) -> F::Output
    where
        F: Handler<usize, io::Error>;

    fn async_read_to_end<F>(&self, sbuf: &mut StreamBuf, handler: F) -> F::Output
    where
        F: Handler<usize, io::Error>,
    {
        let (tx, rx) = handler.channel();
        let sbuf_ptr = sbuf as *mut _;
        match sbuf.prepare(4096) {
            Ok(buf) => self.async_read_some(buf, AsyncReadToEnd::new(self, sbuf_ptr, tx)),
            Err(err) => self.as_ctx().do_dispatch(Failure::new(err, tx)),
        }
        rx.yield_return()
    }

    fn async_read_until<M, F>(&self, sbuf: &mut StreamBuf, cond: M, handler: F) -> F::Output
    where
        M: MatchCond,
        F: Handler<usize, io::Error>,
    {
        let (tx, rx) = handler.channel();
        let sbuf_ptr = sbuf as *mut _;
        match sbuf.prepare(4096) {
            Ok(buf) => self.async_read_some(buf, AsyncReadUntil::new(self, sbuf_ptr, cond, tx)),
            Err(err) => self.as_ctx().do_dispatch(Failure::new(err, tx)),
        }
        rx.yield_return()
    }

    fn async_write_all<M, F>(&self, sbuf: &mut StreamBuf, handler: F) -> F::Output
    where
        M: MatchCond,
        F: Handler<usize, io::Error>,
    {
        let (tx, rx) = handler.channel();
        let sbuf_ptr = sbuf as *mut _;
        let buf = sbuf.as_bytes();
        let len = buf.len();
        self.async_write_some(buf, AsyncWriteAt::new(self, sbuf_ptr, len, tx));
        rx.yield_return()
    }

    fn async_write_until<M, F>(&self, sbuf: &mut StreamBuf, mut cond: M, handler: F) -> F::Output
    where
        M: MatchCond,
        F: Handler<usize, io::Error>,
    {
        let (tx, rx) = handler.channel();
        let sbuf_ptr = sbuf as *mut _;
        let buf = sbuf.as_bytes();
        let len = cond.match_cond(buf).unwrap_or(buf.len());
        self.async_write_some(buf, AsyncWriteAt::new(self, sbuf_ptr, len, tx));
        rx.yield_return()
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
    assert_eq!(sbuf.prepare(100).unwrap().len(), 100);
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
fn test_streambuf_as_bytes() {
    let mut sbuf = StreamBuf::new();
    sbuf.prepare(1000).unwrap();
    sbuf.commit(100);
    assert_eq!(sbuf.as_bytes().len(), 100);
    sbuf.commit(10);
    assert_eq!(sbuf.as_mut_bytes().len(), 110);
}

#[test]
fn test_streambuf_consume() {
    let mut sbuf = StreamBuf::with_max_len(100);
    assert_eq!(sbuf.prepare(1).unwrap().len(), 1);
    assert_eq!(sbuf.prepare(100).unwrap().len(), 100);
    assert_eq!(sbuf.len(), 0);
    sbuf.commit(1);
    assert_eq!(sbuf.len(), 1);
    assert!(sbuf.prepare_exact(100).is_err());
    sbuf.consume(1);
    assert_eq!(sbuf.len(), 0);
    assert!(sbuf.prepare_exact(100).is_ok());
}

#[test]
fn test_streambuf_from_vec() {
    let mut sbuf = StreamBuf::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(sbuf.len(), 10);
    sbuf.consume(9);
    assert_eq!(sbuf.as_bytes()[0], 10);
}

#[test]
fn test_streambuf_read() {
    use std::io::Read;

    let mut sbuf = StreamBuf::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let mut buf = [0; 5];
    assert_eq!(sbuf.read(&mut buf).unwrap(), 5);
    assert_eq!(buf, [1, 2, 3, 4, 5]);
    assert_eq!(sbuf.read(&mut buf).unwrap(), 4);
    assert_eq!(buf, [6, 7, 8, 9, 5]);
    assert!(sbuf.read(&mut buf).is_err());
}

#[test]
fn test_streambuf_write() {
    use std::io::Write;

    let mut sbuf = StreamBuf::with_max_len(9);
    assert_eq!(sbuf.write(&[1, 2, 3, 4, 5]).unwrap(), 5);
    assert_eq!(sbuf.as_bytes(), &[1, 2, 3, 4, 5]);
    assert_eq!(sbuf.write(&[6, 7, 8, 9]).unwrap(), 4);
    assert_eq!(sbuf.as_bytes(), &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert!(sbuf.write(&[1]).is_err());
}

#[test]
fn test_match_cond() {
    assert_eq!((5 as usize).match_cond("hello".as_bytes()), Ok(5));
    assert_eq!((5 as usize).match_cond("hello world".as_bytes()), Ok(5));
    assert_eq!((10 as usize).match_cond("hello".as_bytes()), Err(5));
    assert_eq!('l'.match_cond("hello".as_bytes()), Ok(3));
    assert_eq!('w'.match_cond("hello".as_bytes()), Err(5));
    assert_eq!("lo".match_cond("hello world".as_bytes()), Ok(5));
    assert_eq!("world!".match_cond("hello world".as_bytes()), Err(6));
    assert_eq!("".match_cond("hello".as_bytes()), Err(0));
    assert_eq!("l".match_cond("hello".as_bytes()), Ok(3));
}

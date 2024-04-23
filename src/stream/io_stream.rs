use crate::OsError;
use std::cmp;
use std::ffi::CString;
use std::io;

/// Automatically resizing buffer.
#[derive(Clone, Debug)]
pub struct StreamBuf {
    buf: Vec<u8>,
    max: usize,
    rpos: usize,
    wpos: usize,
}

impl StreamBuf {
    /// Returns a new `StreamBuf`.
    ///
    /// Equivalent to `with_max_len(usize::max_len())`
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::stream::StreamBuf;
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
    /// use asyio::stream::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::with_max_len(1024);
    /// ```
    pub fn with_max_len(max: usize) -> StreamBuf {
        StreamBuf {
            buf: Vec::new(),
            max: max,
            rpos: 0,
            wpos: 0,
        }
    }

    /// Returns an allocated size of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::stream::StreamBuf;
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
    /// use asyio::stream::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::from(vec![1,2,3]);
    /// sbuf.clear();
    /// assert_eq!(sbuf.is_empty(), true);
    /// ```
    pub fn clear(&mut self) {
        self.buf.clear();
        self.rpos = 0;
        self.wpos = 0;
    }

    /// Remove characters from the input sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::stream::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::from(vec![1,2,3]);
    /// assert_eq!(sbuf.len(), 3);
    /// sbuf.consume(3);
    /// assert_eq!(sbuf.len(), 0);
    /// ```
    pub fn consume(&mut self, len: usize) {
        if len < self.len() {
            self.rpos += len
        } else {
            self.clear()
        }
    }

    /// Move characters from the output sequence to the input sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::stream::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::new();
    /// let _ = sbuf.prepare(256);
    /// assert_eq!(sbuf.len(), 0);
    /// sbuf.commit(3);
    /// assert_eq!(sbuf.len(), 3);
    /// ```
    pub fn commit(&mut self, len: usize) {
        if len < (self.buf.len() - self.wpos) {
            self.wpos += len;
        } else {
            self.wpos = self.buf.len();
        }
    }

    /// Returns `true` if the empty buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::stream::StreamBuf;
    ///
    /// let sbuf = StreamBuf::new();
    /// assert!(sbuf.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.rpos == self.wpos
    }

    /// Returns a length of the input sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::stream::StreamBuf;
    ///
    /// let sbuf = StreamBuf::from(vec![1,2,3]);
    /// assert_eq!(sbuf.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.wpos - self.rpos
    }

    /// Returns a maximum length of the `StreamBuf`.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::stream::StreamBuf;
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
    /// use asyio::stream::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::with_max_len(8);
    /// assert_eq!(sbuf.prepare(5).unwrap().len(), 5);
    /// sbuf.commit(5);
    /// assert_eq!(sbuf.prepare(5).unwrap().len(), 3);
    /// ```
    pub fn prepare(&mut self, len: usize) -> Result<&mut [u8], OsError> {
        let mut len = len;
        if len <= (self.max - self.wpos) {
            len += self.wpos;
        } else if 0 != self.rpos || self.wpos != self.max {
            self.buf.drain(..self.rpos);
            self.wpos -= self.rpos;
            self.rpos = 0;
            if len > (self.max - self.wpos) {
                len = self.max;
            } else {
                len += self.wpos;
            }
        } else {
            return Err(OsError::NO_BUFFER_SPACE);
        }

        self.buf.reserve(len);
        unsafe { self.buf.set_len(len) };
        Ok(&mut self.buf[self.wpos..])
    }

    /// Returns a `&mut [u8]` that represents a output sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyio::stream::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::with_max_len(8);
    /// assert_eq!(sbuf.prepare_exact(5).unwrap().len(), 5);
    /// sbuf.commit(5);
    /// assert!(sbuf.prepare_exact(5).is_err());
    /// ```
    pub fn prepare_exact(&mut self, len: usize) -> Result<&mut [u8], OsError> {
        let mut len = len;
        if len <= (self.max - self.wpos) {
        } else if len <= self.rpos {
            self.buf.drain(..self.rpos);
            self.wpos -= self.rpos;
            self.rpos = 0;
        } else {
            return Err(OsError::NO_BUFFER_SPACE);
        }

        len += self.wpos;
        self.buf.reserve(len);
        unsafe { self.buf.set_len(len) };
        Ok(&mut self.buf[self.wpos..])
    }

    /// Returns a `&[u8]` that represents the input sequence.
    pub fn bytes(&self) -> &[u8] {
        &self.buf[self.rpos..self.wpos]
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
            rpos: 0,
            wpos: len,
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
        buf[..len].clone_from_slice(&StreamBuf::bytes(self)[..len]);
        self.consume(len);
        Ok(len)
    }
}

impl io::Write for StreamBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = {
            let wbuf = self.prepare(buf.len()).map_err(Into::<io::Error>::into)?;
            let len = wbuf.len();
            wbuf.clone_from_slice(&buf[..len]);
            len
        };
        self.commit(len);
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn match_cond_bytes(buf: &[u8], head: u8, tail: &[u8]) -> Result<usize, usize> {
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
            match_cond_bytes(buf, self[0], &self[1..])
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
        let mut bytes: [u8; 4] = [0; 4];
        let len = self.encode_utf8(&mut bytes).as_bytes().len();
        match_cond_bytes(buf, bytes[0], &bytes[1..len])
    }
}

impl MatchCond for String {
    fn match_cond(&mut self, buf: &[u8]) -> Result<usize, usize> {
        match_cond_bytes(buf, self.as_bytes()[0], &self.as_bytes()[1..])
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

pub trait IoStream {
    type Error: From<OsError>;

    fn read(&self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    fn write(&self, buf: &[u8]) -> Result<usize, Self::Error>;

    fn read_until<T>(&self, sbuf: &mut StreamBuf, mut cond: T) -> Result<usize, Self::Error>
    where
        T: MatchCond,
    {
        let mut tot = 0;
        loop {
            let buf = sbuf.prepare(4096)?;
            let len = self.read(buf)?;
            match cond.match_cond(&buf[..len]) {
                Ok(len) => {
                    sbuf.commit(len);
                    return Ok(tot + len);
                }
                Err(len) => {
                    sbuf.commit(len);
                    tot += len
                }
            }
        }
    }

    fn write_all(&self, sbuf: &mut StreamBuf) -> Result<usize, Self::Error> {
        let mut tot = 0;
        while sbuf.len() > 0 {
            let len = self.write(sbuf.bytes())?;
            tot += len;
            sbuf.consume(len);
        }
        Ok(tot)
    }
}

pub trait AsyncIoStream {
    type Error: From<OsError>;
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
    assert_eq!(sbuf.prepare(100).unwrap().len(), 30);
    sbuf.commit(70);
    assert_eq!(sbuf.len(), 100);
    sbuf.consume(70);
    assert_eq!(sbuf.len(), 30);
    assert_eq!(sbuf.prepare(100).unwrap().len(), 70);
    assert_eq!(sbuf.len(), 30);
    assert_eq!(sbuf.prepare(200).unwrap().len(), 70);
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
    sbuf.consume(30);
    assert_eq!(sbuf.len(), 40);
    assert_eq!(sbuf.prepare_exact(30).unwrap().len(), 30);
    sbuf.commit(30);
    assert_eq!(sbuf.len(), 70);
    sbuf.consume(70);
    assert_eq!(sbuf.len(), 0);
    assert_eq!(sbuf.prepare_exact(70).unwrap().len(), 70);
    assert_eq!(sbuf.len(), 0);
    assert!(sbuf.prepare_exact(200).is_err());
}

#[test]
fn test_streambuf_as_bytes() {
    let mut sbuf = StreamBuf::new();
    sbuf.prepare(1000).unwrap();
    sbuf.commit(100);
    assert_eq!(sbuf.bytes().len(), 100);
    sbuf.commit(10);
    assert_eq!(sbuf.bytes().len(), 110);
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
fn test_streambuf_commit() {
    let mut sbuf = StreamBuf::new();
    assert_eq!(sbuf.prepare(100).unwrap().len(), 100);
    assert_eq!(sbuf.len(), 0);
    sbuf.commit(1);
    assert_eq!(sbuf.len(), 1);
    sbuf.commit(99);
    assert_eq!(sbuf.len(), 100);
    sbuf.commit(1);
    assert_eq!(sbuf.len(), 100);
}

#[test]
fn test_streambuf_from_vec() {
    let mut sbuf = StreamBuf::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(sbuf.len(), 10);
    sbuf.consume(9);
    assert_eq!(sbuf.bytes()[0], 10);
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
    assert_eq!(sbuf.read(&mut buf).unwrap(), 0);
}

#[test]
fn test_streambuf_write() {
    use std::io::Write;

    let mut sbuf = StreamBuf::with_max_len(9);
    assert_eq!(sbuf.write(&[1, 2, 3, 4, 5]).unwrap(), 5);
    assert_eq!(sbuf.bytes(), &[1, 2, 3, 4, 5]);
    assert_eq!(sbuf.write(&[6, 7, 8, 9]).unwrap(), 4);
    assert_eq!(sbuf.bytes(), &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
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

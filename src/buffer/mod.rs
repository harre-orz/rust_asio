use crate::error::OsError;
use crate::socket_base::Endpoint;
use std::cmp;
use std::ffi::{CString, OsString};
use std::future::Future;
use std::io;
use std::ops::{Deref, DerefMut};

#[derive(Debug)]
#[non_exhaustive]
pub struct ReserveError;

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
    /// use asyncio::buffer::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::new();
    /// ```
    pub const fn new() -> StreamBuf {
        Self::with_max_len(usize::max_value())
    }

    /// Returns a new `StreamBuf` with the max length of the allocatable size.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::buffer::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::with_max_len(1024);
    /// ```
    pub const fn with_max_len(max: usize) -> StreamBuf {
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
    /// use asyncio::buffer::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::new();
    /// assert_eq!(sbuf.capacity(), 0);
    /// ```
    pub const fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Clears the buffer, removing all values.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::buffer::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::from(vec![1,2,3]);
    /// sbuf.clear();
    /// assert_eq!(sbuf.is_empty(), true);
    /// ```
    pub fn clear(&mut self) {
        self.rpos = 0;
        self.wpos = 0;
        self.buf.clear();
    }

    /// Remove characters from the input sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::buffer::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::from(vec![1,2,3]);
    /// assert_eq!(sbuf.len(), 3);
    /// sbuf.consume(3);
    /// assert_eq!(sbuf.len(), 0);
    /// ```
    pub const fn consume(&mut self, len: usize) {
        self.rpos += len;
        if self.rpos >= self.wpos {
            self.rpos = 0;
            self.wpos = 0;
        }
    }

    /// Returns `true` if the empty buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::buffer::StreamBuf;
    ///
    /// let sbuf = StreamBuf::new();
    /// assert!(sbuf.is_empty());
    /// ```
    pub const fn is_empty(&self) -> bool {
        self.rpos == self.wpos
    }

    /// Returns a length of the input sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::buffer::StreamBuf;
    ///
    /// let sbuf = StreamBuf::from(vec![1,2,3]);
    /// assert_eq!(sbuf.len(), 3);
    /// ```
    pub const fn len(&self) -> usize {
        self.wpos - self.rpos
    }

    /// Returns a maximum length of the `StreamBuf`.
    ///
    /// # Examples
    ///
    /// ```
    /// use asyncio::buffer::StreamBuf;
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
    /// use asyncio::buffer::StreamBuf;
    ///
    /// let mut sbuf = StreamBuf::with_max_len(8);
    /// let mut buf = sbuf.prepare(5).unwrap();
    /// assert_eq!(buf.len(), 5);
    /// buf.commit(5);
    ///
    /// ```
    pub fn prepare(&mut self, len: usize) -> Result<StreamBufMut<'_>, ReserveError> {
        let max_len = cmp::min(self.wpos + len, self.max);
        if self.wpos == max_len {
            return Err(ReserveError);
        } else if max_len > self.buf.len() {
            if let Err(_) = self.buf.try_reserve(max_len) {
                return Err(ReserveError);
            } else {
                unsafe {
                    self.buf.set_len(max_len);
                }
            }
        }
        Ok(StreamBufMut(self))
    }

    /// Returns a `&[u8]` that represents the input sequence.
    pub fn as_bytes(&self) -> &[u8] {
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
            max: usize::MAX,
            rpos: 0,
            wpos: len,
        }
    }
}

impl From<String> for StreamBuf {
    fn from(buf: String) -> Self {
        StreamBuf::from(Vec::from(buf.as_bytes()))
    }
}

impl From<OsString> for StreamBuf {
    fn from(buf: OsString) -> Self {
        StreamBuf::from(Vec::from(buf.as_encoded_bytes()))
    }
}

impl From<CString> for StreamBuf {
    fn from(buf: CString) -> Self {
        StreamBuf::from(Vec::from(buf.as_bytes()))
    }
}

impl<'a> From<&'a [u8]> for StreamBuf {
    fn from(buf: &'a [u8]) -> Self {
        StreamBuf::from(Vec::from(buf))
    }
}

impl<'a> From<&'a str> for StreamBuf {
    fn from(buf: &'a str) -> Self {
        StreamBuf::from(Vec::from(buf.as_bytes()))
    }
}

impl io::Read for StreamBuf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = cmp::min(buf.len(), self.as_bytes().len());
        unsafe {
            buf.as_mut_ptr()
                .copy_from_nonoverlapping(self.as_bytes().as_ptr(), len);
        }
        self.consume(len);
        Ok(len)
    }
}

impl io::Write for StreamBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Ok(mut buf_) = self.prepare(buf.len()) {
            let len = buf_.len();
            unsafe {
                buf_.as_bytes_mut()
                    .as_mut_ptr()
                    .copy_from_nonoverlapping(buf.as_ptr(), len);
            }
            buf_.commit(len);
            Ok(len)
        } else {
            Err(OsError::NO_MEMORY.into())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct StreamBufMut<'a>(&'a mut StreamBuf);

impl<'a> StreamBufMut<'a> {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0.buf[self.0.wpos..]
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0.buf[self.0.wpos..]
    }

    /// Move characters from the output sequence to the input sequence.
    ///
    /// # Examples
    ///
    pub const fn commit(self, len: usize) {
        self.0.wpos += len;
        if self.0.wpos > self.0.buf.len() {
            self.0.wpos = self.0.buf.len();
        }
    }
}

impl<'a> Deref for StreamBufMut<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

impl<'a> DerefMut for StreamBufMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_bytes_mut()
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

pub trait MatchCond {
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
        let mut pos = 0;
        loop {
            match cond.match_cond(&sbuf.as_bytes()[pos..]) {
                Ok(len) => return Ok(pos + len),
                Err(len) => {
                    pos += len;
                    if let Ok(mut buf) = sbuf.prepare(4096) {
                        let len = self.read(&mut buf)?;
                        buf.commit(len);
                    } else {
                        return Err(OsError::NO_MEMORY.into());
                    }
                }
            }
        }
    }

    fn write_until<T>(&self, sbuf: &mut StreamBuf, mut cond: T) -> Result<usize, Self::Error>
    where
        T: MatchCond,
    {
        let len = cond.match_cond(sbuf.as_bytes()).unwrap_or(0);
        let mut pos = len;
        while pos > 0 {
            let len = self.write(&sbuf.as_bytes()[..pos])?;
            sbuf.consume(len);
            pos -= len;
        }
        Ok(len)
    }
}

pub trait AsyncIoStream {
    type Error: From<OsError>;

    fn async_read(&self, buf: &mut [u8]) -> impl Future<Output = Result<usize, Self::Error>>;

    fn async_write(&self, buf: &[u8]) -> impl Future<Output = Result<usize, Self::Error>>;

    fn async_read_until<T>(
        &self,
        sbuf: &mut StreamBuf,
        mut cond: T,
    ) -> impl Future<Output = Result<usize, Self::Error>>
    where
        T: MatchCond,
    {
        async move {
            let mut pos = 0;
            loop {
                match cond.match_cond(&sbuf.as_bytes()[pos..]) {
                    Ok(len) => return Ok(pos + len),
                    Err(len) => {
                        pos += len;
                        if let Ok(mut buf) = sbuf.prepare(4096) {
                            let len = self.async_read(buf.as_bytes_mut()).await?;
                            buf.commit(len);
                        } else {
                            return Err(OsError::NO_MEMORY.into());
                        }
                    }
                }
            }
        }
    }

    fn async_write_until<T>(
        &self,
        sbuf: &mut StreamBuf,
        mut cond: T,
    ) -> impl Future<Output = Result<usize, Self::Error>>
    where
        T: MatchCond,
    {
        async move {
            let len = cond.match_cond(sbuf.as_bytes()).unwrap_or(0);
            let mut pos = len;
            while pos > 0 {
                let len = self.async_write(&sbuf.as_bytes()[..pos]).await?;
                sbuf.consume(len);
                pos -= len;
            }
            Ok(len)
        }
    }
}

pub struct MsgBufMut<'a>(&'a mut MsgBuf);

impl<'a> MsgBufMut<'a> {
    pub fn commit<E>(self, len: usize, ep: &E)
    where
        E: Endpoint,
    {
        self.0.commit(len, ep)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.prepare_bytes()
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.0.prepare_bytes()
    }
}

impl<'a> Deref for MsgBufMut<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.prepare_bytes()
    }
}

impl<'a> DerefMut for MsgBufMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.prepare_bytes()
    }
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::MsgBuf;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::MsgBuf;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::MsgBuf;

#[test]
fn test_streambuf() {
    let sbuf = StreamBuf::with_max_len(100);
    assert_eq!(sbuf.len(), 0);
    assert_eq!(sbuf.max_len(), 100);
}

#[test]
fn test_streambuf_prepare() {
    let mut sbuf = StreamBuf::with_max_len(100);

    let buf = sbuf.prepare(100).unwrap();
    assert_eq!(buf.len(), 100);

    buf.commit(70);
    assert_eq!(sbuf.rpos, 0);
    assert_eq!(sbuf.wpos, 70);
    assert_eq!(sbuf.buf.len(), 100);

    let buf = sbuf.prepare(100).unwrap();
    assert_eq!(buf.len(), 30);

    buf.commit(30);
    assert_eq!(sbuf.rpos, 0);
    assert_eq!(sbuf.wpos, 100);
    assert_eq!(sbuf.buf.len(), 100);

    sbuf.consume(70);
    assert_eq!(sbuf.rpos, 70);
    assert_eq!(sbuf.wpos, 100);
    assert_eq!(sbuf.buf.len(), 100);
}

#[test]
fn test_streambuf_as_bytes() {
    let mut sbuf = StreamBuf::new();

    sbuf.prepare(1000).unwrap().commit(100);
    assert_eq!(sbuf.as_bytes().len(), 100);

    sbuf.prepare(1000).unwrap().commit(10);
    assert_eq!(sbuf.as_bytes().len(), 110);
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
    assert_eq!(sbuf.read(&mut buf).unwrap(), 0);
}

#[test]
fn test_streambuf_write() {
    use std::io::Write;

    let mut sbuf = StreamBuf::with_max_len(9);
    assert_eq!(sbuf.rpos, 0);
    assert_eq!(sbuf.wpos, 0);
    assert_eq!(sbuf.buf.len(), 0);

    let len = sbuf.write(&[1, 2, 3, 4, 5]).unwrap();
    assert_eq!(len, 5);
    assert_eq!(sbuf.as_bytes(), &[1, 2, 3, 4, 5]);
    assert_eq!(sbuf.rpos, 0);
    assert_eq!(sbuf.wpos, 5);
    assert_eq!(sbuf.buf.len(), 5);

    let len = sbuf.write(&[6, 7, 8, 9, 10]).unwrap();
    assert_eq!(len, 4);
    assert_eq!(sbuf.as_bytes(), &[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(sbuf.rpos, 0);
    assert_eq!(sbuf.wpos, 9);
    assert_eq!(sbuf.buf.len(), 9);
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

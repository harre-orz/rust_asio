use std::io;
use std::fmt;
use std::cmp;
use std::mem;
use std::hash;
use std::slice;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use traits::{Protocol, SockAddr};
use libc::{AF_UNIX, sockaddr, sockaddr_un};
use sa_ops::{sockaddr_eq, sockaddr_cmp, sockaddr_hash};

/// The endpoint of UNIX domain socket.
#[derive(Clone)]
pub struct LocalEndpoint<P: Protocol> {
    len: usize,
    sun: sockaddr_un,
    _marker: PhantomData<P>,
}

impl<P: Protocol> LocalEndpoint<P> {

    /// Returns a `LocalEndpoint`.
    ///
    /// # Example
    ///
    /// ```
    /// use asyncio::local::LocalStreamEndpoint;
    ///
    /// assert!(LocalStreamEndpoint::new("file name").is_ok());
    /// assert!(LocalStreamEndpoint::new("file name very long                                                                                                  ").is_err());
    /// ```
    pub fn new<T>(path_name: T) -> io::Result<LocalEndpoint<P>>
        where T: AsRef<str>
    {
        match CString::new(path_name.as_ref()) {
            Ok(ref s) if s.as_bytes().len() < (mem::size_of::<sockaddr_un>() - 2) => {
                let src = s.as_bytes_with_nul();
                let mut ep = LocalEndpoint {
                    len: src.len() + 2,
                    sun: unsafe { mem::uninitialized() },
                    _marker: PhantomData,
                };
                ep.sun.sun_family = AF_UNIX as u16;
                let dst = unsafe { slice::from_raw_parts_mut(ep.sun.sun_path.as_mut_ptr() as *mut u8, src.len()) };
                dst.clone_from_slice(src);
                Ok(ep)
            }
            _ => Err(io::Error::new(io::ErrorKind::Other, "invalid argument")),
        }
    }

    /// Returns a path_name associated with the endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// use asyncio::local::LocalStreamEndpoint;
    ///
    /// let ep = LocalStreamEndpoint::new("foo.sock").unwrap();
    /// assert_eq!(ep.path(), "foo.sock");
    /// ```
    pub fn path(&self) -> &str {
        let cstr = unsafe { CStr::from_ptr(self.sun.sun_path.as_ptr()) };
        cstr.to_str().unwrap()
    }
}

impl<P: Protocol> SockAddr for LocalEndpoint<P> {
    fn as_sockaddr(&self) -> &sockaddr {
        unsafe { mem::transmute(&self.sun) }
    }

    fn as_mut_sockaddr(&mut self) -> &mut sockaddr {
        unsafe { mem::transmute(&mut self.sun) }
    }

    fn capacity(&self) -> usize {
        mem::size_of_val(&self.sun)
    }

    fn size(&self) -> usize {
        self.len
    }

    unsafe fn resize(&mut self, size: usize) {
        debug_assert!(size <= self.capacity());
        self.len = size;
    }
}

impl<P: Protocol> Eq for LocalEndpoint<P> {
}

impl<P: Protocol> PartialEq for LocalEndpoint<P> {
    fn eq(&self, other: &Self) -> bool {
        sockaddr_eq(self, other)
    }
}

impl<P: Protocol> Ord for LocalEndpoint<P> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        sockaddr_cmp(self, other)
    }
}

impl<P: Protocol> PartialOrd for LocalEndpoint<P> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<P: Protocol> hash::Hash for LocalEndpoint<P> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        sockaddr_hash(self, state)
    }
}

impl<P: Protocol> fmt::Display for LocalEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.path())
    }
}

impl<P: Protocol> fmt::Debug for LocalEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }
}

/// A category of an local protocol.
pub trait LocalProtocol : Protocol {
}

mod dgram;
pub use self::dgram::*;

mod stream;
pub use self::stream::*;

mod seq_packet;
pub use self::seq_packet::*;

mod connect_pair;
pub use self::connect_pair::*;


#[test]
fn test_local_endpoint_limit() {
    assert_eq!(LocalStreamEndpoint::new("foo").unwrap(), LocalStreamEndpoint::new("foo").unwrap());
    assert!(LocalDgramEndpoint::new("").is_ok());
    let s = "01234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789";
    assert!(LocalSeqPacketEndpoint::new(&s[..103]).is_ok());
    assert!(LocalSeqPacketEndpoint::new(&s[..108]).is_err());
}

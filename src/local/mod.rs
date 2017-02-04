use prelude::{Protocol, SockAddr};
use core::Socket;
use ffi::{AF_UNIX, SockAddrImpl, sockaddr_un};
use error::{invalid_argument};

use std::io;
use std::fmt;
use std::mem;
use std::slice;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;

/// The endpoint of UNIX domain socket.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LocalEndpoint<P: Protocol> {
    sun: SockAddrImpl<sockaddr_un>,
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
                    sun: SockAddrImpl::new(AF_UNIX, src.len() + 2),
                    _marker: PhantomData,
                };
                let dst = unsafe { slice::from_raw_parts_mut(
                    ep.sun.sun_path.as_mut_ptr() as *mut u8,
                    src.len()
                ) };
                dst.clone_from_slice(src);
                Ok(ep)
            }
            _ => Err(invalid_argument()),
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
    type SockAddr = sockaddr_un;

    fn as_ref(&self) -> &Self::SockAddr {
        &*self.sun
    }

    unsafe fn as_mut(&mut self) -> &mut Self::SockAddr {
        &mut *self.sun
    }

    fn capacity(&self) -> usize {
        self.sun.capacity()
    }

    fn size(&self) -> usize {
        self.sun.size()
    }

    unsafe fn resize(&mut self, size: usize) {
        debug_assert!(size <= self.capacity());
        self.sun.resize(size)
    }
}

impl<P: Protocol> fmt::Display for LocalEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.path())
    }
}

/// A category of an local protocol.
pub trait LocalProtocol : Protocol {
    type Socket : Socket<Self>;
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
    assert_eq!(LocalStreamEndpoint::new("foo").unwrap(),
               LocalStreamEndpoint::new("foo").unwrap());
    assert!(LocalDgramEndpoint::new("").is_ok());
    let s = "01234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789";
    assert!(LocalSeqPacketEndpoint::new(&s[..103]).is_ok());
    assert!(LocalSeqPacketEndpoint::new(&s[..108]).is_err());
}

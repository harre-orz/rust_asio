use std::io;
use std::mem;
use std::fmt;
use std::cmp;
use std::hash;
use std::slice;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use {Protocol, Endpoint};
use backbone::{AF_LOCAL, sockaddr, sockaddr_un, endpoint_eq, endpoint_cmp, endpoint_hash};

const UNIX_PATH_MAX: usize = 108;

/// A category of an local protocol.
pub trait LocalProtocol : Protocol {
}

#[derive(Clone)]
pub struct LocalEndpoint<P> {
    len: usize,
    sun: sockaddr_un,
    marker: PhantomData<P>,
}

impl<P> LocalEndpoint<P> {
    pub fn new<T: Into<Vec<u8>>>(path: T) -> io::Result<LocalEndpoint<P>> {
        match CString::new(path) {
            Ok(ref s) if s.as_bytes().len() < UNIX_PATH_MAX => {
                let src = s.as_bytes_with_nul();
                let mut ep = LocalEndpoint {
                    len: src.len() + 2,
                    sun: unsafe { mem::uninitialized() },
                    marker: PhantomData,
                };
                ep.sun.sun_family = AF_LOCAL as u16;
                let dst = unsafe { slice::from_raw_parts_mut(ep.sun.sun_path.as_mut_ptr() as *mut u8, src.len()) };
                dst.clone_from_slice(src);
                Ok(ep)
            }
            _ =>
                Err(io::Error::new(io::ErrorKind::Other, "Unsupported pathname")),
        }
    }

    pub fn path(&self) -> &str {
        let cstr = unsafe { CStr::from_ptr(self.sun.sun_path.as_ptr()) };
        cstr.to_str().unwrap()
    }
}

impl<P: Protocol> Endpoint for LocalEndpoint<P> {
    type SockAddr = sockaddr;

    fn as_sockaddr(&self) -> &Self::SockAddr {
        unsafe { mem::transmute(&self.sun) }
    }

    fn as_mut_sockaddr(&mut self) -> &mut Self::SockAddr {
        unsafe { mem::transmute(&mut self.sun) }
    }

    fn capacity(&self) -> usize {
        mem::size_of::<sockaddr_un>()
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
        endpoint_eq(self, other)
    }
}

impl<P: Protocol> Ord for LocalEndpoint<P> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        endpoint_cmp(self, other)
    }
}

impl<P: Protocol> PartialOrd for LocalEndpoint<P> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<P: Protocol> hash::Hash for LocalEndpoint<P> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        endpoint_hash(self, state)
    }
}

impl<P> fmt::Display for LocalEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.path())
    }
}

impl<P> fmt::Debug for LocalEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\"{}\"", self)
    }
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
    assert!(LocalSeqPacketEndpoint::new(&s[..UNIX_PATH_MAX-1]).is_ok());
    assert!(LocalSeqPacketEndpoint::new(&s[..UNIX_PATH_MAX]).is_err());
}

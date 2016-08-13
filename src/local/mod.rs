use std::io;
use std::mem;
use std::fmt;
use std::cmp;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use {Protocol, Endpoint};
use backbone::{AF_LOCAL, sockaddr_un, c_char, memcmp};

const UNIX_PATH_MAX: usize = 108;

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
                let mut ep = LocalEndpoint {
                    len: mem::size_of::<sockaddr_un>(),
                    sun: unsafe { mem::zeroed() },
                    marker: PhantomData,
                };
                ep.sun.sun_family = AF_LOCAL as u16;
                for (src, dst) in s.as_bytes_with_nul().iter().zip(ep.sun.sun_path.iter_mut()) {
                    *dst = *src as c_char;
                }
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
    type SockAddr = sockaddr_un;

    fn as_sockaddr(&self) -> &Self::SockAddr {
        &self.sun
    }

    fn as_mut_sockaddr(&mut self) -> &mut Self::SockAddr {
        &mut self.sun
    }

    fn size(&self) -> usize {
        self.len
    }

    fn resize(&mut self, size: usize) {
        self.len = cmp::min(size, self.capacity())
    }

    fn capacity(&self) -> usize {
        mem::size_of::<Self::SockAddr>()
    }
}

impl<P> Eq for LocalEndpoint<P> {
}

impl<P> PartialEq for LocalEndpoint<P> {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            memcmp(
                mem::transmute(&self.sun),
                mem::transmute(&other.sun),
                mem::size_of::<sockaddr_un>()
            ) == 0
        }
    }
}

impl<P> Ord for LocalEndpoint<P> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        let cmp = unsafe {
            memcmp(
                mem::transmute(&self.sun),
                mem::transmute(&other.sun),
                mem::size_of::<sockaddr_un>()
            )
        };
        if cmp == 0 {
            cmp::Ordering::Equal
        } else if cmp < 0 {
            cmp::Ordering::Less
        } else {
            cmp::Ordering::Greater
        }
    }
}

impl<P> PartialOrd for LocalEndpoint<P> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<P> fmt::Display for LocalEndpoint<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.path())
    }
}

mod dgram;
pub use self::dgram::*;

mod stream;
pub use self::stream::*;

mod seq_packet;
pub use self::seq_packet::*;

#[test]
fn test_local_endpoint() {
    assert!(LocalStreamEndpoint::new("foo").unwrap() == LocalStreamEndpoint::new("foo").unwrap());
    assert!(LocalDgramEndpoint::new("").is_ok());
    let s = "01234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789";
    assert!(LocalSeqPacketEndpoint::new(&s[..UNIX_PATH_MAX-1]).is_ok());
    assert!(LocalSeqPacketEndpoint::new(&s[..UNIX_PATH_MAX]).is_err());
}

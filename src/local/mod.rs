use std::io;
use std::mem;
use std::fmt;
use std::cmp;
use std::path::Path;
use std::marker::PhantomData;
use {Protocol, AsSockAddr};
use ops::*;

fn str2c_char(src: &str, dst: &mut [c_char]) {
    let len = cmp::min(dst.len()-1, src.len());
    for (dst, src) in dst.iter_mut().zip(src.chars()) {
        *dst = src as c_char;
    };
    dst[len] = '\0' as c_char;
}

fn c_char2string(src: &[c_char]) -> String {
    let mut s = String::new();
    for c in src {
        if *c == 0 {
            break;
        }
        s.push((*c as u8) as char);
    }
    s
}

#[derive(Clone)]
pub struct LocalEndpoint<P: Protocol> {
    len: usize,
    sun: sockaddr_un,
    marker: PhantomData<P>,
}

impl<P: Protocol> LocalEndpoint<P> {
    pub fn new<T: AsRef<Path>>(path: T) -> io::Result<LocalEndpoint<P>> {
        match path.as_ref().to_str() {
            Some(s) if s.len() < UNIX_PATH_MAX => {
                let mut ep = LocalEndpoint {
                    len: mem::size_of::<sockaddr_un>(),
                    sun: unsafe { mem::zeroed() },
                    marker: PhantomData,
                };
                ep.sun.sun_family = AF_LOCAL as u16;
                str2c_char(&s, &mut ep.sun.sun_path);
                Ok(ep)
            },
            _ => Err(io::Error::new(io::ErrorKind::Other, "Unsupported pathname")), // EFGIB
        }
    }

    pub fn path(&self) -> String {
        c_char2string(&self.sun.sun_path)
    }
}

impl<P: Protocol> AsSockAddr for LocalEndpoint<P> {
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

impl<P: Protocol> Eq for LocalEndpoint<P> {
}

impl<P: Protocol> PartialEq for LocalEndpoint<P> {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            c_memcmp(
                mem::transmute(&self.sun),
                mem::transmute(&other.sun),
                mem::size_of::<sockaddr_un>()
            ) == 0
        }
    }
}

impl<P: Protocol> Ord for LocalEndpoint<P> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        let cmp = unsafe {
            c_memcmp(
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

impl<P: Protocol> PartialOrd for LocalEndpoint<P> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<P: Protocol> fmt::Display for LocalEndpoint<P> {
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

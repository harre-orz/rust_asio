use std::io;
use std::mem;
use std::fmt;
use std::cmp;
use std::path::Path;
use std::marker::PhantomData;
use socket::Protocol;
use ops::*;

#[derive(Clone)]
pub struct LocalEndpoint<P: Protocol> {
    sun: sockaddr_un,
    marker: PhantomData<P>,
}

impl<P: Protocol> LocalEndpoint<P> {
    pub fn new<T: AsRef<Path>>(path: T) -> io::Result<LocalEndpoint<P>> {
        match path.as_ref().to_str() {
            Some(s) if s.len() < UNIX_PATH_MAX => {
                let mut ep = LocalEndpoint {
                    sun: unsafe { mem::zeroed() },
                    marker: PhantomData,
                };
                ep.sun.sun_family = AF_UNIX as u16;
                str2c_char(&s, &mut ep.sun.sun_path);
                Ok(ep)
            },
            _ => Err(io::Error::new(io::ErrorKind::Other, "Unsupported pathname")),
        }
    }

    pub fn path(&self) -> String {
        c_char2string(&self.sun.sun_path)
    }
}

impl<P: Protocol> AsRawSockAddr for LocalEndpoint<P> {
    fn as_raw_sockaddr(&self) -> &RawSockAddrType {
        unsafe { mem::transmute(&self.sun) }
    }

    fn as_mut_raw_sockaddr(&mut self) -> &mut RawSockAddrType {
        unsafe { mem::transmute(&mut self.sun) }
    }

    fn raw_socklen(&self) -> RawSockLenType {
        mem::size_of_val(&self.sun) as RawSockLenType
    }
}

unsafe impl<P: Protocol> Send for LocalEndpoint<P> {}

impl<P: Protocol> Eq for LocalEndpoint<P> {
}

impl<P: Protocol> PartialEq for LocalEndpoint<P> {
    fn eq(&self, other: &Self) -> bool {
        raw_sockaddr_eq(self, other)
    }
}

impl<P: Protocol> Ord for LocalEndpoint<P> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        raw_sockaddr_cmp(self, other)
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

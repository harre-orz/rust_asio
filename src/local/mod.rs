use ffi::{AF_UNIX, EINVAL, SockAddr, sockaddr_un, socketpair, error};
use core::{IoContext, SocketContext, PairBox, Tx, Rx};
use prelude::Protocol;

use std::io;
use std::mem;
use std::slice;
use std::path::Path;
use std::marker::PhantomData;
use std::ffi::{CString, OsStr};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::SocketAddr;

/// The endpoint of UNIX domain socket.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LocalEndpoint<P> {
    sun: SockAddr<sockaddr_un>,
    _marker: PhantomData<P>,
}

impl<P> LocalEndpoint<P> {
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
        where T: AsRef<Path>
    {
        match CString::new(path_name.as_ref().as_os_str().as_bytes()) {
            Ok(ref s) if s.as_bytes().len() < (mem::size_of::<sockaddr_un>() - 2) => {
                let src = s.as_bytes_with_nul();
                let mut ep = LocalEndpoint {
                    sun: SockAddr::new(AF_UNIX, (src.len() + 2) as u8),
                    _marker: PhantomData,
                };
                let dst = unsafe { slice::from_raw_parts_mut(
                    ep.sun.sa.sun_path.as_mut_ptr() as *mut u8,
                    src.len()
                ) };
                dst.clone_from_slice(src);
                Ok(ep)
            },
            _ => Err(io::Error::from_raw_os_error(EINVAL)),
        }
    }

    pub fn is_unnamed(&self) -> bool {
        self.sun.sa.sun_path[0] == 0
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
    pub fn as_pathname(&self) -> Option<&Path> {
        if !self.is_unnamed() {
            Some(Path::new(OsStr::from_bytes(unsafe {
                slice::from_raw_parts(self.sun.sa.sun_path.as_ptr() as *const u8,
                                      (self.sun.size() - 3) as usize)
            })))
        } else {
            None
        }
    }
}

impl<P> From<SocketAddr> for LocalEndpoint<P> {
    fn from(sa: SocketAddr) -> Self {
        LocalEndpoint::new(sa.as_pathname().unwrap()).unwrap()
    }
}

/// A category of an local protocol.
pub trait LocalProtocol : Protocol {
    type Tx : Tx<Self>;
    type Rx : Rx<Self>;
}

/// Returns a pair of connected UNIX domain sockets.
///
/// # Example
///
/// ```
/// use std::thread;
/// use asyncio::{IoContext, Stream};
/// use asyncio::local::{LocalStream, LocalStreamSocket, connect_pair};
///
/// const MESSAGE: &'static str = "hello";
///
/// let ctx = &IoContext::new().unwrap();
/// let (tx, rx) = connect_pair(ctx, LocalStream).unwrap();
///
/// let thrd = thread::spawn(move|| {
///     let mut buf = [0; 32];
///     let len = rx.read_some(&mut buf).unwrap();
///     assert_eq!(len, MESSAGE.len());
///     assert_eq!(&buf[..len], MESSAGE.as_bytes());
/// });
///
/// tx.write_some(MESSAGE.as_bytes()).unwrap();
/// thrd.join().unwrap();
/// ```
pub fn connect_pair<P>(ctx: &IoContext, pro: P) -> io::Result<(P::Tx, P::Rx)>
    where P: LocalProtocol,
{
    let (tx, rx) = socketpair(&pro).map_err(error)?;
    let (tx, _) = PairBox::new(SocketContext::new(ctx, pro, tx));
    let (_, rx) = PairBox::new(SocketContext::new(ctx, pro, rx));
    Ok((P::Tx::from_ctx(tx), P::Rx::from_ctx(rx)))
}

mod dgram;
pub use self::dgram::*;

mod stream;
pub use self::stream::*;

mod seq_packet;
pub use self::seq_packet::*;

// #[test]
// fn test_local_endpoint_limit() {
//     assert_eq!(LocalStreamEndpoint::new("foo").unwrap(),
//                LocalStreamEndpoint::new("foo").unwrap());
//     assert!(LocalDgramEndpoint::new("").is_ok());
//     let s = "01234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789";
//     assert!(LocalSeqPacketEndpoint::new(&s[..103]).is_ok());
//     assert!(LocalSeqPacketEndpoint::new(&s[..108]).is_err());
// }

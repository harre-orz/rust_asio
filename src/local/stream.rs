use ffi::{sockaddr, socklen_t, AF_UNIX, SOCK_STREAM};
use prelude::{Endpoint, Protocol};
use socket_listener::SocketListener;
use stream_socket::StreamSocket;
use local::{LocalEndpoint, LocalProtocol};

use std::fmt;
use std::mem;

/// The stream-oriented UNIX domain protocol.
///
/// # Example
/// Create a server and client sockets.
///
/// ```rust,no_run
/// use asyncio::{IoContext, Endpoint};
/// use asyncio::local::{LocalStream, LocalStreamEndpoint, LocalStreamSocket, LocalStreamListener};
///
/// let ctx = &IoContext::new().unwrap();
/// let ep = LocalStreamEndpoint::new("example.sock").unwrap();
///
/// let sv = LocalStreamListener::new(ctx, LocalStream).unwrap();
/// sv.bind(&ep).unwrap();
/// sv.listen().unwrap();
///
/// let cl = LocalStreamSocket::new(ctx, ep.protocol()).unwrap();
/// cl.connect(&ep).unwrap();
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct LocalStream;

impl Protocol for LocalStream {
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(&self) -> i32 {
        AF_UNIX
    }

    fn socket_type(&self) -> i32 {
        SOCK_STREAM
    }

    fn protocol_type(&self) -> i32 {
        0
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl LocalProtocol for LocalStream {}

impl Endpoint<LocalStream> for LocalEndpoint<LocalStream> {
    fn protocol(&self) -> LocalStream {
        LocalStream
    }

    fn as_ptr(&self) -> *const sockaddr {
        &self.sun as *const _ as *const _
    }

    fn as_mut_ptr(&mut self) -> *mut sockaddr {
        &mut self.sun as *mut _ as *mut _
    }

    fn capacity(&self) -> socklen_t {
        self.sun.capacity() as socklen_t
    }

    fn size(&self) -> socklen_t {
        self.sun.size() as socklen_t
    }

    unsafe fn resize(&mut self, size: socklen_t) {
        debug_assert!(size <= self.capacity());
        self.sun.resize(size as u8)
    }
}

impl fmt::Debug for LocalEndpoint<LocalStream> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}:{:?})", self.protocol(), self.as_pathname())
    }
}

/// The stream-oriented UNIX domain endpoint type
pub type LocalStreamEndpoint = LocalEndpoint<LocalStream>;

/// The stream-oriented UNIX domain socket type.
pub type LocalStreamSocket = StreamSocket<LocalStream>;

/// The stream-oriented UNIX domain listener type.
pub type LocalStreamListener = SocketListener<LocalStream, LocalStreamSocket>;

#[test]
fn test_getsockname_local() {
    use core::IoContext;
    use local::*;

    use std::fs;

    let ctx = &IoContext::new().unwrap();
    let ep = LocalStreamEndpoint::new(".asio_foo.sock").unwrap();
    println!("{:?}", ep.as_pathname().unwrap());
    let _ = fs::remove_file(ep.as_pathname().unwrap());
    let soc = LocalStreamSocket::new(ctx, ep.protocol()).unwrap();
    soc.bind(&ep).unwrap();
    assert_eq!(soc.local_endpoint().unwrap(), ep);
    assert_eq!(soc.local_endpoint().unwrap(), ep);
    let _ = fs::remove_file(ep.as_pathname().unwrap());
}

#[test]
fn test_format() {
    use core::IoContext;

    let _ctx = &IoContext::new().unwrap();
    println!("{:?}", LocalStream);
    println!("{:?}", LocalStreamEndpoint::new("foo/bar").unwrap());
}

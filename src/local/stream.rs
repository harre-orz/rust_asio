use std::mem;
use libc::{AF_UNIX, SOCK_STREAM};
use traits::{Protocol, Endpoint};
use stream_socket::StreamSocket;
use socket_listener::{SocketListener};
use super::{LocalProtocol, LocalEndpoint};

/// The stream-oriented UNIX domain protocol.
///
/// # Example
/// Create a server and client sockets.
///
/// ```rust,no_run
/// use asyncio::IoService;
/// use asyncio::local::{LocalStream, LocalStreamEndpoint, LocalStreamSocket, LocalStreamListener};
///
/// let io = &IoService::new();
/// let ep = LocalStreamEndpoint::new("example.sock").unwrap();
///
/// let sv = LocalStreamListener::new(io, LocalStream).unwrap();
/// sv.bind(&ep).unwrap();
/// sv.listen().unwrap();
///
/// let cl = LocalStreamSocket::new(io, LocalStream).unwrap();
/// cl.connect(&ep).unwrap();
/// ```
#[derive(Clone, Eq, PartialEq, Debug)]
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

impl LocalProtocol for LocalStream {
}

impl Endpoint<LocalStream> for LocalEndpoint<LocalStream> {
    fn protocol(&self) -> LocalStream {
        LocalStream
    }
}

/// The stream-oriented UNIX domain endpoint type
pub type LocalStreamEndpoint = LocalEndpoint<LocalStream>;

/// The stream-oriented UNIX domain socket type.
pub type LocalStreamSocket = StreamSocket<LocalStream>;

/// The stream-oriented UNIX domain listener type.
pub type LocalStreamListener = SocketListener<LocalStream>;

#[test]
fn test_stream() {
    assert!(LocalStream == LocalStream);
}

#[test]
fn test_getsockname_local() {
    use IoService;
    use super::*;
    use std::fs;

    let io = IoService::new();
    let soc = LocalStreamSocket::new(&io, LocalStream).unwrap();
    let ep = LocalStreamEndpoint::new("/tmp/asio_foo.sock").unwrap();
    let _ = fs::remove_file(ep.path());
    soc.bind(&ep).unwrap();
    assert_eq!(soc.local_endpoint().unwrap(), ep);
    let _ = fs::remove_file(ep.path());
}

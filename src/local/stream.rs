use ffi::{AF_UNIX, SOCK_STREAM};
use prelude::Protocol;
use socket_base::{Tx, Rx};
use socket_builder::SocketBuilder;
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
#[derive(Clone, Copy)]
pub struct LocalStream;

impl LocalEndpoint<LocalStream> {
    pub fn protocol(&self) -> LocalStream {
        LocalStream
    }
}

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
    type Tx = StreamSocket<LocalStream, Tx>;
    type Rx = StreamSocket<LocalStream, Rx>;
}

impl fmt::Debug for LocalStream {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StreamDgram")
    }
}

impl fmt::Debug for LocalEndpoint<LocalStream> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LocalEndpoint(Stream:\"{}\")", self)
    }
}

/// The stream-oriented UNIX domain endpoint type
pub type LocalStreamEndpoint = LocalEndpoint<LocalStream>;

pub type LocalStreamBuilder = SocketBuilder<LocalStream, StreamSocket<LocalStream, Tx>, StreamSocket<LocalStream, Rx>>;

/// The stream-oriented UNIX domain listener type.
pub type LocalStreamListener = SocketListener<LocalStream, StreamSocket<LocalStream, Tx>, StreamSocket<LocalStream, Rx>>;

/// The stream-oriented UNIX domain socket type.
pub type LocalStreamRxSocket = StreamSocket<LocalStream, Rx>;

/// The stream-oriented UNIX domain socket type.
pub type LocalStreamTxSocket = StreamSocket<LocalStream, Tx>;


#[test]
fn test_stream() {
    assert!(LocalStream == LocalStream);
}

#[test]
fn test_getsockname_local() {
    use core::IoContext;
    use local::*;

    use std::fs;

    let ctx = &IoContext::new().unwrap();
    let ep = LocalStreamEndpoint::new("/tmp/asio_foo.sock").unwrap();
    let soc = LocalStreamSocket::new(ctx, ep.protocol()).unwrap();
    let _ = fs::remove_file(ep.path());
    soc.bind(&ep).unwrap();
    assert_eq!(soc.local_endpoint().unwrap(), ep);
    let _ = fs::remove_file(ep.path());
}

#[test]
fn test_format() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    println!("{:?}", LocalStream);
    println!("{:?}", LocalStreamEndpoint::new("foo/bar").unwrap());
    println!("{:?}", LocalStreamSocket::new(ctx, LocalStream).unwrap());
}

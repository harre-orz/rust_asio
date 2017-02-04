use prelude::{Protocol, Endpoint};
use ffi::{AF_UNIX, SOCK_DGRAM};
use dgram_socket::DgramSocket;
use local::{LocalProtocol, LocalEndpoint};

use std::fmt;
use std::mem;

/// The datagram-oriented UNIX domain protocol.
///
/// # Example
/// Create a server and client sockets.
///
/// ```rust,no_run
/// use asyncio::{IoContext, Endpoint};
/// use asyncio::local::{LocalDgram, LocalDgramEndpoint, LocalDgramSocket};
///
/// let ctx = &IoContext::new().unwrap();
/// let ep = LocalDgramEndpoint::new("example.sock").unwrap();
///
/// let sv = LocalDgramSocket::new(ctx, LocalDgram).unwrap();
/// sv.bind(&ep).unwrap();
///
/// let cl = LocalDgramSocket::new(ctx, ep.protocol()).unwrap();
/// cl.connect(&ep).unwrap();
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct LocalDgram;

impl Protocol for LocalDgram {
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(&self) -> i32 {
        AF_UNIX
    }

    fn socket_type(&self) -> i32 {
        SOCK_DGRAM
    }

    fn protocol_type(&self) -> i32 {
        0
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl LocalProtocol for LocalDgram {
    type Socket = DgramSocket<Self>;
}

impl Endpoint<LocalDgram> for LocalEndpoint<LocalDgram> {
    fn protocol(&self) -> LocalDgram {
        LocalDgram
    }
}

impl fmt::Debug for LocalDgram {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LocalDgram")
    }
}

impl fmt::Debug for LocalEndpoint<LocalDgram> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LocalEndpoint(Dgram:\"{}\")", self)
    }
}

/// The datagram-oriented UNIX domain endpoint type.
pub type LocalDgramEndpoint = LocalEndpoint<LocalDgram>;

/// The datagram-oriented UNIX domain socket type.
pub type LocalDgramSocket = DgramSocket<LocalDgram>;

#[test]
fn test_dgram() {
    assert!(LocalDgram == LocalDgram);
}

#[test]
fn test_format() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    println!("{:?}", LocalDgram);
    println!("{:?}", LocalDgramEndpoint::new("foo/bar").unwrap());
    println!("{:?}", LocalDgramSocket::new(ctx, LocalDgram).unwrap());
}

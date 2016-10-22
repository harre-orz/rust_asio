use std::mem;
use {Protocol, Endpoint, DgramSocket};
use libc::{AF_UNIX, SOCK_DGRAM};
use super::{LocalProtocol, LocalEndpoint};

/// The datagram-oriented UNIX domain protocol.
///
/// # Example
/// Create a server and client sockets.
///
/// ```rust,no_run
/// use asyncio::IoService;
/// use asyncio::local::{LocalDgram, LocalDgramEndpoint, LocalDgramSocket};
///
/// let io = &IoService::new();
/// let ep = LocalDgramEndpoint::new("example.sock").unwrap();
///
/// let sv = LocalDgramSocket::new(io, LocalDgram).unwrap();
/// sv.bind(&ep).unwrap();
///
/// let cl = LocalDgramSocket::new(io, LocalDgram).unwrap();
/// cl.connect(&ep).unwrap();
/// ```
#[derive(Clone, Eq, PartialEq, Debug)]
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
}

impl Endpoint<LocalDgram> for LocalEndpoint<LocalDgram> {
    fn protocol(&self) -> LocalDgram {
        LocalDgram
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

use prelude::{Protocol, Endpoint};
use ffi::{AF_UNIX, SOCK_SEQPACKET};
use seq_packet_socket::{SeqPacketSocket};
use socket_listener::{SocketListener};
use local::{LocalProtocol, LocalEndpoint};

use std::fmt;
use std::mem;

/// The seq-packet protocol.
///
/// # Example
/// Create a server and client sockets.
///
/// ```rust,no_run
/// use asyncio::{IoContext, Endpoint};
/// use asyncio::local::{LocalSeqPacket, LocalSeqPacketEndpoint, LocalSeqPacketSocket, LocalSeqPacketListener};
///
/// let ctx = &IoContext::new().unwrap();
/// let ep = LocalSeqPacketEndpoint::new("example.sock").unwrap();
///
/// let sv = LocalSeqPacketListener::new(ctx, LocalSeqPacket).unwrap();
/// sv.bind(&ep).unwrap();
/// sv.listen().unwrap();
///
/// let cl = LocalSeqPacketSocket::new(ctx, ep.protocol()).unwrap();
/// cl.connect(&ep).unwrap();
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct LocalSeqPacket;

impl Protocol for LocalSeqPacket {
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(&self) -> i32 {
        AF_UNIX
    }

    fn socket_type(&self) -> i32 {
        SOCK_SEQPACKET
    }

    fn protocol_type(&self) -> i32 {
        0
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        mem::uninitialized()
    }
}

impl LocalProtocol for LocalSeqPacket {
    type Socket = SeqPacketSocket<Self>;
}

impl Endpoint<LocalSeqPacket> for LocalEndpoint<LocalSeqPacket> {
    fn protocol(&self) -> LocalSeqPacket {
        LocalSeqPacket
    }
}

impl fmt::Debug for LocalSeqPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LocalSeqPacket")
    }
}

impl fmt::Debug for LocalEndpoint<LocalSeqPacket> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "LocalEndpoint(SeqPacket:\"{}\")", self)
    }
}

/// The seq-packet endpoint type.
pub type LocalSeqPacketEndpoint = LocalEndpoint<LocalSeqPacket>;

/// The seq-packet socket type.
pub type LocalSeqPacketSocket = SeqPacketSocket<LocalSeqPacket>;

/// The seq-packet listener type.
pub type LocalSeqPacketListener = SocketListener<LocalSeqPacket, SeqPacketSocket<LocalSeqPacket>>;

#[test]
fn test_seq_packet() {
    assert!(LocalSeqPacket == LocalSeqPacket);
}

#[test]
#[cfg(target_os = "linux")]
fn test_format() {
    use core::IoContext;

    let ctx = &IoContext::new().unwrap();
    println!("{:?}", LocalSeqPacket);
    println!("{:?}", LocalSeqPacketEndpoint::new("foo/bar").unwrap());
    println!("{:?}", LocalSeqPacketSocket::new(ctx, LocalSeqPacket).unwrap());
}

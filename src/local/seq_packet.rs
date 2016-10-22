use std::mem;
use traits::{Protocol, Endpoint};
use seq_packet_socket::{SeqPacketSocket};
use socket_listener::{SocketListener};
use libc::{AF_UNIX, SOCK_SEQPACKET};
use super::{LocalProtocol, LocalEndpoint};

/// The seq-packet protocol.
///
/// # Example
/// Create a server and client sockets.
///
/// ```rust,no_run
/// use asyncio::IoService;
/// use asyncio::local::{LocalSeqPacket, LocalSeqPacketEndpoint, LocalSeqPacketSocket, LocalSeqPacketListener};
///
/// let io = &IoService::new();
/// let ep = LocalSeqPacketEndpoint::new("example.sock").unwrap();
///
/// let sv = LocalSeqPacketListener::new(io, LocalSeqPacket).unwrap();
/// sv.bind(&ep).unwrap();
/// sv.listen().unwrap();
///
/// let cl = LocalSeqPacketSocket::new(io, LocalSeqPacket).unwrap();
/// cl.connect(&ep).unwrap();
/// ```
#[derive(Clone, Eq, PartialEq, Debug)]
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
}

impl Endpoint<LocalSeqPacket> for LocalEndpoint<LocalSeqPacket> {
    fn protocol(&self) -> LocalSeqPacket {
        LocalSeqPacket
    }
}

/// The seq-packet endpoint type.
pub type LocalSeqPacketEndpoint = LocalEndpoint<LocalSeqPacket>;

/// The seq-packet socket type.
pub type LocalSeqPacketSocket = SeqPacketSocket<LocalSeqPacket>;

/// The seq-packet listener type.
pub type LocalSeqPacketListener = SocketListener<LocalSeqPacket>;

#[test]
fn test_seq_packet() {
    assert!(LocalSeqPacket == LocalSeqPacket);
}

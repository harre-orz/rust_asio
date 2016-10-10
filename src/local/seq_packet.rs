use std::mem;
use traits::{Protocol, Endpoint};
use seq_packet_socket::{SeqPacketSocket};
use socket_listener::{SocketListener};
use libc::{AF_UNIX, SOCK_SEQPACKET};
use super::{LocalProtocol, LocalEndpoint};

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

pub type LocalSeqPacketEndpoint = LocalEndpoint<LocalSeqPacket>;

pub type LocalSeqPacketSocket = SeqPacketSocket<LocalSeqPacket>;

pub type LocalSeqPacketListener = SocketListener<LocalSeqPacket>;

#[test]
fn test_seq_packet() {
    assert!(LocalSeqPacket == LocalSeqPacket);
}

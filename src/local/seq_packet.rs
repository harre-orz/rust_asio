use std::mem;
use {Protocol, SeqPacketSocket, SocketListener};
use backbone::{AF_LOCAL, SOCK_SEQPACKET};
use super::{LocalProtocol, LocalEndpoint};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct LocalSeqPacket;

impl Protocol for LocalSeqPacket {
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(&self) -> i32 {
        AF_LOCAL
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

impl LocalEndpoint<LocalSeqPacket> {
    pub fn protocol(&self) -> LocalSeqPacket {
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

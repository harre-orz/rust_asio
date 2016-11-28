use traits::{Protocol, SockAddr, Endpoint};
use seq_packet_socket::{SeqPacketSocket};
use socket_listener::{SocketListener};
use libc::SOCK_SEQPACKET;
use super::GenericEndpoint;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct GenericSeqPacket {
    family: i32,
    protocol: i32,
    capacity: usize,
}

impl Protocol for GenericSeqPacket {
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_SEQPACKET
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        GenericEndpoint::default(self.capacity, self.protocol)
    }
}

impl Endpoint<GenericSeqPacket> for GenericEndpoint<GenericSeqPacket> {
    fn protocol(&self) -> GenericSeqPacket {
        GenericSeqPacket {
            family: self.as_sockaddr().sa_family as i32,
            protocol: self.protocol,
            capacity: self.capacity(),
        }
    }
}

pub type GenericSeqPacketEndpoint = GenericEndpoint<GenericSeqPacket>;

pub type GenericSeqPacketSocket = SeqPacketSocket<GenericSeqPacket>;

pub type GenericSeqPacketListener = SocketListener<GenericSeqPacket, SeqPacketSocket<GenericSeqPacket>>;

//

use super::GenericEndpoint;
use dgram_socket::DgramSocket;
use libc;
use socket_base::Protocol;
use socket_listener::SocketListener;
use std::mem;

pub struct GenericSeqPacket {
    family: i32,
    protocol: i32,
}

impl GenericSeqPacket {
    pub fn new(family: i32, protocol: i32) -> Self {
        GenericSeqPacket {
            family: family,
            protocol: protocol,
        }
    }
}

impl Protocol for GenericSeqPacket {
    type Endpoint = GenericEndpoint<Self>;

    type Socket = DgramSocket<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_STREAM as _
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        let data: Box<[u8]> = Box::new([0; mem::size_of::<libc::sockaddr_storage>()]);
        GenericEndpoint::new(data)
    }
}

pub type GenericSeqPacketEndpoint = GenericEndpoint<GenericSeqPacket>;

pub type GenericSeqPacketSocket = DgramSocket<GenericSeqPacket>;

pub type GenericSeqPacketListener = SocketListener<GenericSeqPacket>;

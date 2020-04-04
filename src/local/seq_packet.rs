//

use super::LocalEndpoint;
use dgram_socket::DgramSocket;
use libc;
use socket_base::Protocol;
use socket_listener::SocketListener;
use std::mem::MaybeUninit;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalSeqPacket;

impl LocalEndpoint<LocalSeqPacket> {
    pub fn protocol(&self) -> LocalSeqPacket {
        LocalSeqPacket
    }
}

impl Protocol for LocalSeqPacket {
    type Endpoint = LocalEndpoint<Self>;
    type Socket = DgramSocket<Self>;

    fn family_type(&self) -> i32 {
        libc::AF_LOCAL
    }

    fn socket_type(&self) -> i32 {
        libc::SOCK_SEQPACKET
    }

    fn protocol_type(&self) -> i32 {
        0
    }

    fn uninit(&self) -> MaybeUninit<Self::Endpoint> {
        MaybeUninit::uninit()
    }
}

pub type LocalSeqPacketEndpoint = LocalEndpoint<LocalSeqPacket>;

pub type LocalSeqPacketSocket = DgramSocket<LocalSeqPacket>;

pub type LocalSeqPacketListener = SocketListener<LocalSeqPacket>;

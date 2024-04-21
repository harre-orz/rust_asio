use super::{LocalEndpoint, LocalProtocol};
use crate::dgram::SeqPacketSocket;
use crate::listener::{ConnectedSocket, IntoConnectedSocket, SocketListener};
use crate::{AddressFamily, Protocol, SocketType};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct SeqPacket;

impl Protocol for SeqPacket {
    type Type = LocalProtocol;
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        AddressFamily::UNIX
    }

    fn socket_type(self) -> SocketType {
        SocketType::SEQPACKET
    }

    fn protocol_type(self) -> Self::Type {
        LocalProtocol
    }
}

pub type LocalSeqPacketEndpoint = LocalEndpoint<SeqPacket>;
pub type LocalSeqPacketSocket = SeqPacketSocket<SeqPacket>;
pub type LocalSeqPacketListener = SocketListener<SeqPacket, LocalSeqPacketSocket>;

impl IntoConnectedSocket for LocalSeqPacketListener {
    type Socket = LocalSeqPacketSocket;

    fn into_connected_socket(&self, soc: ConnectedSocket) -> Self::Socket {
        LocalSeqPacketSocket::new_priv(self.as_ctx().clone(), self.protocol(), soc.0)
    }
}

#[test]
fn test_dgram() {}

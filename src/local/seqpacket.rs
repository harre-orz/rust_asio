use super::{LocalEndpoint, LocalProtocol};
use crate::dgram::SeqPacketSocket;
use crate::listener::{ConnectedSocket, IntoConnectedSocket, SocketListener};
use crate::socket_base::{AddressFamily, Protocol, SocketType};

/// The seq-packet protocol.
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

/// The seq-packet endpoint type.
pub type LocalSeqPacketEndpoint = LocalEndpoint<SeqPacket>;

/// The seq-packet socket type.
pub type LocalSeqPacketSocket = SeqPacketSocket<SeqPacket>;

/// The seq-packet listener type.
pub type LocalSeqPacketListener = SocketListener<SeqPacket, LocalSeqPacketSocket>;

impl IntoConnectedSocket for LocalSeqPacketListener {
    type Socket = LocalSeqPacketSocket;

    fn into_connected_socket(&self, conn: ConnectedSocket) -> Self::Socket {
        LocalSeqPacketSocket::new_priv(conn.ctx, self.protocol(), conn.soc)
    }
}

#[test]
fn test_dgram() {}

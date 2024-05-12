use super::{LocalEndpoint, LocalProtocol};
use crate::dgram::SeqPacketSocketBuilder;
use crate::dgram::{AsyncSeqPacketSocket, SeqPacketSocket};
use crate::error::OsError;
use crate::ffi::{ConnectedSocket, IntoSocket};
use crate::listener::{AsyncSocketListener, SocketListener};
use crate::socket_base::{AddressFamily, Protocol, SocketType};
use crate::IoContext;

/// The seq-packet protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct SeqPacket;

impl SeqPacket {
    pub fn connect<E>(ctx: &IoContext, ep: E) -> Result<SeqPacketSocket<Self>, OsError>
    where
        E: AsRef<LocalEndpoint<Self>>,
    {
        SeqPacketSocketBuilder::new(ctx, Self)?.connect(ep)
    }
}

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

impl IntoSocket for SocketListener<SeqPacket, SeqPacketSocket<SeqPacket>> {
    type Socket = SeqPacketSocket<SeqPacket>;

    fn into_socket(&self, soc: ConnectedSocket) -> Self::Socket {
        Self::Socket::new_priv(soc, self.protocol(), self.as_ctx())
    }
}

impl IntoSocket for AsyncSocketListener<SeqPacket, AsyncSeqPacketSocket<SeqPacket>> {
    type Socket = AsyncSeqPacketSocket<SeqPacket>;

    fn into_socket(&self, soc: ConnectedSocket) -> Self::Socket {
        SeqPacketSocket::new_priv(soc, self.protocol(), self.as_ctx()).into()
    }
}

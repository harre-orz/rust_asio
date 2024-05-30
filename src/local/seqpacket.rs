use super::{LocalEndpoint, LocalProtocol};
use crate::dgram::SeqPacketSocketBuilder;
use crate::dgram::{AsyncSeqPacketSocket, SeqPacketSocket};
use crate::error::OsError;
use crate::ffi::{self, ConnectedSocket, IntoSocket};
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

    pub fn socketpair(
        ctx: &IoContext,
    ) -> Result<(SeqPacketSocket<Self>, SeqPacketSocket<Self>), OsError> {
        let (s1, s2) = ffi::socketpair(Self)?;
        Ok((
            SeqPacketSocket::new_priv(ctx, s1, Self),
            SeqPacketSocket::new_priv(ctx, s2, Self),
        ))
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

impl IntoSocket for SocketListener<SeqPacket> {
    type Socket = SeqPacketSocket<SeqPacket>;

    fn into_socket(&self, soc: ConnectedSocket) -> Self::Socket {
        Self::Socket::new_priv(self.as_ctx(), soc, self.protocol())
    }
}

impl IntoSocket for AsyncSocketListener<SeqPacket> {
    type Socket = AsyncSeqPacketSocket<SeqPacket>;

    fn into_socket(&self, soc: ConnectedSocket) -> Self::Socket {
        SeqPacketSocket::new_priv(self.as_ctx(), soc, self.protocol()).into()
    }
}

use super::{LocalEndpoint, LocalProtocol};
use crate::dgram::{AsyncSeqPacketSocket, SeqPacketSocket};
use crate::error::OsError;
use crate::ffi::{self, Socket};
use crate::listener::{AsyncSocketListener, ConnectedSocket, SocketListener};
use crate::socket_base::{AddressFamily, Protocol, SocketType};
use crate::IoContext;

/// The seq-packet protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalSeqPacket;

impl LocalSeqPacket {
    pub fn connect(ctx: &IoContext, ep: &LocalEndpoint<Self>) -> Result<SeqPacketSocket<Self>, OsError> {
        let soc = SeqPacketSocket::new(ctx, Self)?;
        soc.connect(ep)
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

impl Protocol for LocalSeqPacket {
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

/// The seq-packet-oriented UNIX domain socket type.
pub type LocalSeqPacketSocket = SeqPacketSocket<LocalSeqPacket>;

/// The seq-packet-oriented UNIX domain endpoint type.
pub type LocalSeqPacketEndpoint = LocalEndpoint<LocalSeqPacket>;

/// The seq-packet-oriented UNIX domain listener type.
pub type LocalSeqPacketListener = SocketListener<LocalSeqPacket>;

impl ConnectedSocket for LocalSeqPacketListener {
    type Socket = LocalSeqPacketSocket;

    fn socket(&self, soc: Socket) -> Self::Socket {
        LocalSeqPacketSocket::new_priv(self.as_ctx(), soc, self.protocol())
    }
}

impl ConnectedSocket for AsyncSocketListener<LocalSeqPacket> {
    type Socket = AsyncSeqPacketSocket<LocalSeqPacket>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        SeqPacketSocket::new_priv(self.as_ctx(), soc, self.protocol()).into()
    }
}

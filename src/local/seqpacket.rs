use crate::IoContext;
use crate::error::Result;
use crate::local::{LocalEndpoint, LocalProtocol};
use crate::seqpacket_socket::{AsyncSeqPacketSocket, SeqPacketSocket, SeqPacketSocketBuilder};
use crate::sockaddr::AddressFamily;
use crate::socket::{Socket, SocketType};
use crate::socket_base::{EndpointRef, Protocol};
use crate::socket_listener::{
    AsyncSocketListener, ConnectedSocket, SocketListener, SocketListenerBuilder,
};

/// The seq-packet protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalSeqPacket;

impl LocalSeqPacket {
    #[cfg(unix)]
    pub fn new_pair(ctx: &IoContext) -> Result<(SeqPacketSocket<Self>, SeqPacketSocket<Self>)> {
        let (s1, s2) = Socket::socketpair(Self)?;
        Ok((
            SeqPacketSocket::new_impl(ctx.clone(), s1, Self),
            SeqPacketSocket::new_impl(ctx.clone(), s2, Self),
        ))
    }
}

impl Protocol for LocalSeqPacket {
    type Endpoint = LocalEndpoint<Self>;
    type Type = LocalProtocol;

    fn new(_: &EndpointRef<Self::Endpoint>, _: Self::Type) -> Self {
        Self
    }

    fn family_type(self) -> AddressFamily {
        AddressFamily::AF_LOCAL
    }

    fn socket_type(self) -> SocketType {
        SocketType::SOCK_SEQPACKET
    }

    fn protocol_type(self) -> Self::Type {
        LocalProtocol
    }
}

impl SeqPacketSocket<LocalSeqPacket> {
    pub fn new(ctx: &IoContext) -> SeqPacketSocketBuilder<LocalSeqPacket> {
        SeqPacketSocketBuilder::new_impl(ctx.clone(), LocalProtocol)
    }
}

impl SocketListener<LocalSeqPacket> {
    pub fn new(ctx: &IoContext) -> SocketListenerBuilder<LocalSeqPacket> {
        SocketListenerBuilder::new_impl(ctx.clone(), LocalProtocol)
    }
}

impl ConnectedSocket<LocalSeqPacket> for SocketListener<LocalSeqPacket> {
    type Socket = SeqPacketSocket<LocalSeqPacket>;

    fn connected(&self, soc: Socket, pro: LocalSeqPacket) -> Self::Socket {
        Self::Socket::new_impl(self.as_ctx().clone(), soc, pro)
    }
}

impl ConnectedSocket<LocalSeqPacket> for AsyncSocketListener<LocalSeqPacket> {
    type Socket = AsyncSeqPacketSocket<LocalSeqPacket>;

    fn connected(&self, soc: Socket, pro: LocalSeqPacket) -> Self::Socket {
        SeqPacketSocket::new_impl(self.as_ctx().clone(), soc, pro).into()
    }
}

/// The seq-packet-oriented UNIX domain endpoint type.
pub type LocalSeqPacketEndpoint = LocalEndpoint<LocalSeqPacket>;

/// The seq-packet-oriented UNIX domain socket type.
pub type LocalSeqPacketSocket = SeqPacketSocket<LocalSeqPacket>;

/// The seq-packet-oriented UNIX domain listener type.
pub type LocalSeqPacketListener = SocketListener<LocalSeqPacket>;

/// The seq-packet-oriented UNIX domain async socket type.
pub type AsyncLocalSeqPacketSocket = AsyncSeqPacketSocket<LocalSeqPacket>;

/// The seq-packet-oriented UNIX domain async listener type.
pub type AsyncLocalSeqPacketListener = AsyncSocketListener<LocalSeqPacket>;

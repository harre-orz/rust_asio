use crate::IoContext;
use crate::dgram_socket::{AsyncDgramSocket, DgramSocket};
use crate::generic::GenericEndpoint;
use crate::seqpacket_socket::{AsyncSeqPacketSocket, SeqPacketSocket, SeqPacketSocketBuilder};
use crate::sockaddr::{AddressFamily, SockAddr};
use crate::socket::{Socket, SocketType};
use crate::socket_base::Protocol;
use crate::socket_listener::{AsyncSocketListener, ConnectedSocket, SocketListener};

#[derive(Copy, Clone)]
pub struct GenericSeqPacket<T>(AddressFamily, T);

impl<T> GenericSeqPacket<T> {
    pub const fn new(family: AddressFamily, protocol: T) -> Self {
        Self(family, protocol)
    }
}

impl<T> Protocol for GenericSeqPacket<T>
where
    T: Copy + Into<i32>,
{
    type Type = T;
    type Endpoint = GenericEndpoint<Self>;

    fn new(address_family: AddressFamily, protocol: Self::Type) -> Self {
        Self(address_family, protocol)
    }

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::SOCK_SEQPACKET
    }

    fn protocol_type(self) -> T {
        self.1
    }
}

impl<T> GenericEndpoint<GenericSeqPacket<T>>
where
    T: Copy + Into<i32>,
{
    pub fn protocol(&self, pro: T) -> GenericSeqPacket<T> {
        GenericSeqPacket(self.ss.address_family(), pro)
    }
}

impl<T> SeqPacketSocket<GenericSeqPacket<T>>
where
    T: Copy + Into<i32>,
{
    pub fn new(ctx: &IoContext, pro: T) -> SeqPacketSocketBuilder<GenericSeqPacket<T>> {
        SeqPacketSocketBuilder::new_impl(ctx.clone(), pro)
    }
}

impl<T> ConnectedSocket for SocketListener<GenericSeqPacket<T>>
where
    T: Copy + Into<i32>,
{
    type Socket = SeqPacketSocket<GenericSeqPacket<T>>;

    fn connected(&self, soc: Socket) -> Self::Socket {
        Self::Socket::new_impl(self.as_ctx().clone(), soc)
    }
}

impl<T> ConnectedSocket for AsyncSocketListener<GenericSeqPacket<T>>
where
    T: Copy + Into<i32>,
{
    type Socket = AsyncSeqPacketSocket<GenericSeqPacket<T>>;

    fn connected(&self, soc: Socket) -> Self::Socket {
        SeqPacketSocket::new_impl(self.as_ctx().clone(), soc).into()
    }
}

pub type GenericSeqPacketEndpoint<T> = GenericEndpoint<GenericSeqPacket<T>>;
pub type GenericSeqPacketSocket<T> = DgramSocket<GenericSeqPacket<T>>;
pub type GenericSeqPacketListener<T> = SocketListener<GenericSeqPacket<T>>;
pub type AsyncGenericSeqPacketSocket<T> = AsyncDgramSocket<GenericSeqPacket<T>>;
pub type AsyncGenericSeqPacketListener<T> = SocketListener<GenericSeqPacket<T>>;

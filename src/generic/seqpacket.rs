use crate::IoContext;
use crate::dgram_socket::{AsyncDgramSocket, DgramSocket};
use crate::generic::GenericEndpoint;
use crate::socket::Socket;
use crate::seqpacket_socket::{AsyncSeqPacketSocket, SeqPacketSocket, SeqPacketSocketBuilder};
use crate::sockaddr::AddressFamily;
use crate::socket_base::{EndpointRef, Protocol, SocketType};
use crate::socket_listener::{AsyncSocketListener, ConnectedSocket, SocketListener};

#[derive(Copy, Clone)]
pub struct GenericSeqPacket<T>(AddressFamily, T);

impl<T> Protocol for GenericSeqPacket<T>
where
    T: Copy + Into<i32> + 'static,
{
    type Endpoint = GenericEndpoint<Self>;
    type Type = T;

    fn new(ep: &EndpointRef<Self::Endpoint>, protocol: Self::Type) -> Self {
        Self(AddressFamily::from_sockaddr(ep.sockaddr_ref()), protocol)
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

impl<T> SeqPacketSocket<GenericSeqPacket<T>>
where
    T: Copy + Into<i32> + 'static,
{
    pub fn new(ctx: &IoContext, pro: T) -> SeqPacketSocketBuilder<GenericSeqPacket<T>> {
        SeqPacketSocketBuilder::new_impl(ctx.clone(), pro)
    }
}

impl<T> ConnectedSocket<GenericSeqPacket<T>> for SocketListener<GenericSeqPacket<T>>
where
    T: Copy + Into<i32> + 'static,
{
    type Socket = SeqPacketSocket<GenericSeqPacket<T>>;

    fn connected(&self, soc: Socket, pro: GenericSeqPacket<T>) -> Self::Socket {
        Self::Socket::new_impl(soc, pro)
    }
}

impl<T> ConnectedSocket<GenericSeqPacket<T>> for AsyncSocketListener<GenericSeqPacket<T>>
where
    T: Copy + Into<i32> + 'static,
{
    type Socket = SeqPacketSocket<GenericSeqPacket<T>>;

    fn connected(&self, soc: Socket, pro: GenericSeqPacket<T>) -> Self::Socket {
        SeqPacketSocket::new_impl(soc, pro).into()
    }
}

pub type GenericSeqPacketEndpoint<T> = GenericEndpoint<GenericSeqPacket<T>>;
pub type GenericSeqPacketSocket<T> = DgramSocket<GenericSeqPacket<T>>;
pub type GenericSeqPacketListener<T> = SocketListener<GenericSeqPacket<T>>;
pub type AsyncGenericSeqPacketSocket<T> = AsyncDgramSocket<GenericSeqPacket<T>>;
pub type AsyncGenericSeqPacketListener<T> = SocketListener<GenericSeqPacket<T>>;

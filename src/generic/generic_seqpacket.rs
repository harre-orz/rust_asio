use super::GenericEndpoint;
use crate::dgram::{AsyncSeqPacketSocket, SeqPacketSocket};
use crate::ffi::Socket;
use crate::listener::{AsyncSocketListener, ConnectedSocket};
use crate::socket_base::{AddressFamily, IntoProtocolType, Protocol, SocketType};

pub struct GenericSeqPacket<P>(AddressFamily, P);

impl<P: IntoProtocolType> Clone for GenericSeqPacket<P> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<P: IntoProtocolType> Copy for GenericSeqPacket<P> {}

impl<P: IntoProtocolType> GenericSeqPacket<P> {
    pub const fn new(family: AddressFamily, protocol: P) -> Self {
        Self(family, protocol)
    }
}

impl<P> Protocol for GenericSeqPacket<P>
where
    P: IntoProtocolType,
{
    type Type = P;
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::RAW
    }

    fn protocol_type(self) -> Self::Type {
        self.1
    }
}

impl<P> ConnectedSocket for SeqPacketSocket<GenericSeqPacket<P>>
where
    P: IntoProtocolType,
{
    type Socket = SeqPacketSocket<GenericSeqPacket<P>>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        SeqPacketSocket::new_priv(self.as_ctx(), soc, self.protocol())
    }
}

impl<P> ConnectedSocket for AsyncSocketListener<GenericSeqPacket<P>>
where
    P: IntoProtocolType,
{
    type Socket = AsyncSeqPacketSocket<GenericSeqPacket<P>>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        SeqPacketSocket::new_priv(self.as_ctx(), soc, self.protocol()).into()
    }
}

use super::GenericEndpoint;
use crate::socket_base::{AddressFamily, IntoProtocolType, Protocol, SocketType};
use crate::ffi::Socket;
use crate::listener::{AsyncSocketListener, ConnectedSocket, SocketListener};
use crate::dgram::SeqPacketSocket;

pub struct SeqPacket<P>(AddressFamily, P);

impl<P: IntoProtocolType> Clone for SeqPacket<P> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<P: IntoProtocolType> Copy for SeqPacket<P> {}

impl<P: IntoProtocolType> SeqPacket<P> {
    pub const fn new(family: AddressFamily, protocol: P) -> Self {
        Self(family, protocol)
    }
}

impl<P> Protocol for SeqPacket<P>
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

impl<P> ConnectedSocket for SocketListener<SeqPacket<P>>
where
     P: IntoProtocolType,
{
    type Socket = SeqPacketSocket<SeqPacket<P>>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        SeqPacketSocket::new_priv(self.as_ctx(), soc, self.protocol())
    }
}

impl<P> ConnectedSocket for AsyncSocketListener<SeqPacket<P>>
where
     P: IntoProtocolType,
{
    type Socket = SeqPacketSocket<SeqPacket<P>>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        SeqPacketSocket::new_priv(self.as_ctx(), soc, self.protocol())
    }
}

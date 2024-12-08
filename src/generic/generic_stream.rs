use super::GenericEndpoint;
use crate::ffi::Socket;
use crate::listener::{AsyncSocketListener, ConnectedSocket, SocketListener};
use crate::socket_base::{AddressFamily, IntoProtocolType, Protocol, SocketType};
use crate::stream::{AsyncStreamSocket, StreamSocket};

pub struct GenericStream<P>(AddressFamily, P);

impl<P> Clone for GenericStream<P>
where
    P: IntoProtocolType,
{
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<P> Copy for GenericStream<P> where P: IntoProtocolType {}

impl<P> GenericStream<P>
where
    P: IntoProtocolType,
{
    pub const fn new(family: AddressFamily, protocol: P) -> Self {
        Self(family, protocol)
    }
}

impl<P> Protocol for GenericStream<P>
where
    P: IntoProtocolType,
{
    type Type = P;
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::STREAM
    }

    fn protocol_type(self) -> Self::Type {
        self.1
    }
}

impl<P> ConnectedSocket for SocketListener<GenericStream<P>>
where
    P: IntoProtocolType,
{
    type Socket = StreamSocket<GenericStream<P>>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        StreamSocket::new_priv(self.as_ctx(), soc, self.protocol())
    }
}

impl<P> ConnectedSocket for AsyncSocketListener<GenericStream<P>>
where
    P: IntoProtocolType,
{
    type Socket = AsyncStreamSocket<GenericStream<P>>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        StreamSocket::new_priv(self.as_ctx(), soc, self.protocol()).into()
    }
}

use crate::IoContext;
use crate::dgram_socket::{AsyncDgramSocket, DgramSocket};
use crate::generic::GenericEndpoint;
use crate::primitive::Socket;
use crate::sockaddr::AddressFamily;
use crate::socket_base::{EndpointRef, Protocol, SocketType};
use crate::socket_listener::{AsyncSocketListener, ConnectedSocket, SocketListener};
use crate::stream_socket::{StreamSocket, StreamSocketBuilder};

#[derive(Copy, Clone)]
pub struct GenericStream<T>(AddressFamily, T);

impl<T> Protocol for GenericStream<T>
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
        SocketType::SOCK_STREAM
    }

    fn protocol_type(self) -> T {
        self.1
    }
}

impl<T> StreamSocket<GenericStream<T>>
where
    T: Copy + Into<i32> + 'static,
{
    pub fn new(ctx: &IoContext, pro: T) -> StreamSocketBuilder<GenericStream<T>> {
        StreamSocketBuilder::new_impl(ctx.clone(), pro)
    }
}

impl<T> ConnectedSocket<GenericStream<T>> for SocketListener<GenericStream<T>>
where
    T: Copy + Into<i32> + 'static,
{
    type Socket = StreamSocket<GenericStream<T>>;

    fn connected(&self, soc: Socket, pro: GenericStream<T>) -> Self::Socket {
        StreamSocket::new_impl(self.as_ctx().clone(), soc, pro)
    }
}

impl<T> ConnectedSocket<GenericStream<T>> for AsyncSocketListener<GenericStream<T>>
where
    T: Copy + Into<i32> + 'static,
{
    type Socket = StreamSocket<GenericStream<T>>;

    fn connected(&self, soc: Socket, pro: GenericStream<T>) -> Self::Socket {
        StreamSocket::new_impl(self.as_ctx().clone(), soc, pro)
    }
}

pub type GenericStreamEndpoint<T> = GenericEndpoint<GenericStream<T>>;
pub type GenericStreamSocket<T> = DgramSocket<GenericStream<T>>;
pub type GenericStreamListener<T> = SocketListener<GenericStream<T>>;
pub type AsyncGenericStreamSocket<T> = AsyncDgramSocket<GenericStream<T>>;
pub type AsyncGenericStreamListener<T> = SocketListener<GenericStream<T>>;

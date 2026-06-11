use crate::IoContext;
#[cfg(unix)]
use crate::error::Result;
use crate::local::{LocalEndpoint, LocalProtocol};
use crate::sockaddr::AddressFamily;
use crate::socket::Socket;
use crate::socket_base::{EndpointRef, Protocol, SocketType};
use crate::socket_listener::{
    AsyncSocketListener, ConnectedSocket, SocketListener, SocketListenerBuilder,
};
use crate::stream_socket::{AsyncStreamSocket, StreamSocket, StreamSocketBuilder};

/// The stream-oriented UNIX domain protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalStream;

impl LocalStream {
    #[cfg(unix)]
    pub fn new_pair(ctx: &IoContext) -> Result<(StreamSocket<Self>, StreamSocket<Self>)> {
        let (s1, s2) = Socket::socketpair(ctx, Self)?;
        Ok((
            StreamSocket::new_impl(s1, Self),
            StreamSocket::new_impl(s2, Self),
        ))
    }
}

impl Protocol for LocalStream {
    type Endpoint = LocalEndpoint<Self>;
    type Type = LocalProtocol;

    fn new(_: &EndpointRef<Self::Endpoint>, _: Self::Type) -> Self {
        Self
    }

    fn family_type(self) -> AddressFamily {
        AddressFamily::AF_LOCAL
    }

    fn socket_type(self) -> SocketType {
        SocketType::SOCK_STREAM
    }

    fn protocol_type(self) -> Self::Type {
        LocalProtocol
    }
}

impl StreamSocket<LocalStream> {
    pub fn new(ctx: &IoContext) -> StreamSocketBuilder<LocalStream> {
        StreamSocketBuilder::new_impl(ctx.clone(), LocalProtocol)
    }
}

impl SocketListener<LocalStream> {
    pub fn new(ctx: &IoContext) -> SocketListenerBuilder<LocalStream> {
        SocketListenerBuilder::new_impl(ctx.clone(), LocalProtocol)
    }
}

impl ConnectedSocket<LocalStream> for AsyncSocketListener<LocalStream> {
    type Socket = StreamSocket<LocalStream>;

    fn connected(&self, soc: Socket, pro: LocalStream) -> Self::Socket {
        StreamSocket::new_impl(soc, pro)
    }
}

impl ConnectedSocket<LocalStream> for SocketListener<LocalStream> {
    type Socket = StreamSocket<LocalStream>;

    fn connected(&self, soc: Socket, pro: LocalStream) -> Self::Socket {
        Self::Socket::new_impl(soc, pro)
    }
}

/// The stream-oriented UNIX domain endpoint type
pub type LocalStreamEndpoint = LocalEndpoint<LocalStream>;

/// The stream-oriented UNIX domain socket type
pub type LocalStreamSocket = StreamSocket<LocalStream>;

/// The stream-oriented UNIX domain listener type
pub type LocalStreamListener = SocketListener<LocalStream>;

/// The stream-oriented UNIX domain async socket type
pub type AsyncLocalStreamSocket = AsyncStreamSocket<LocalStream>;

/// The stream-oriented UNIX domain async listener type
pub type AsyncLocalStreamListener = AsyncSocketListener<LocalStream>;

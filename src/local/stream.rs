use crate::IoContext;
use crate::error::OsError;
use crate::local::{LocalEndpoint, LocalProtocol};
use crate::sockaddr::AddressFamily;
use crate::socket::{Socket, SocketType};
use crate::socket_base::Protocol;
use crate::socket_listener::{
    AsyncSocketListener, ConnectedSocket, SocketListener, SocketListenerBuilder,
};
use crate::stream_socket::{AsyncStreamSocket, StreamSocket, StreamSocketBuilder};
use std::result;

type Result<T> = result::Result<T, OsError>;

/// The stream-oriented UNIX domain protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalStream;

impl LocalStream {
    #[cfg(unix)]
    pub fn new_pair(ctx: &IoContext) -> Result<(StreamSocket<Self>, StreamSocket<Self>)> {
        let (s1, s2) = Socket::socketpair(Self)?;
        Ok((
            StreamSocket::new_impl(ctx.clone(), s1),
            StreamSocket::new_impl(ctx.clone(), s2),
        ))
    }
}

impl Protocol for LocalStream {
    type Type = LocalProtocol;
    type Endpoint = LocalEndpoint<Self>;

    fn new(_: AddressFamily, _: Self::Type) -> Self {
        Self
    }

    fn family_type(self) -> AddressFamily {
        AddressFamily::AF_LOCAL
    }

    fn socket_type(self) -> SocketType {
        SocketType::SOCK_STREAM.into()
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

impl ConnectedSocket for AsyncSocketListener<LocalStream> {
    type Socket = AsyncStreamSocket<LocalStream>;

    fn connected(&self, soc: Socket) -> Self::Socket {
        StreamSocket::new_impl(self.as_ctx().clone(), soc).into()
    }
}

impl ConnectedSocket for SocketListener<LocalStream> {
    type Socket = StreamSocket<LocalStream>;

    fn connected(&self, soc: Socket) -> Self::Socket {
        Self::Socket::new_impl(self.as_ctx().clone(), soc)
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

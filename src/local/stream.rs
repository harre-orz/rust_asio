use super::{LocalEndpoint, LocalProtocol};
use crate::ffi::{ConnectedSocket, IntoSocket};
use crate::listener::{AsyncSocketListener, SocketListener};
use crate::socket_base::{AddressFamily, Protocol, SocketType};
use crate::stream::{AsyncStreamSocket, StreamSocket};

/// The stream-oriented UNIX domain protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Stream;

impl Protocol for Stream {
    type Type = LocalProtocol;
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        AddressFamily::UNIX
    }

    fn socket_type(self) -> SocketType {
        SocketType::STREAM
    }

    fn protocol_type(self) -> Self::Type {
        LocalProtocol
    }
}

/// The stream-oriented UNIX domain endpoint type
pub type LocalStreamEndpoint = LocalEndpoint<Stream>;

impl IntoSocket for SocketListener<Stream, StreamSocket<Stream>> {
    type Socket = StreamSocket<Stream>;

    fn into_socket(&self, soc: ConnectedSocket) -> Self::Socket {
        Self::Socket::new_priv(soc, self.protocol(), self.as_ctx())
    }
}

impl IntoSocket for AsyncSocketListener<Stream, AsyncStreamSocket<Stream>> {
    type Socket = AsyncStreamSocket<Stream>;

    fn into_socket(&self, soc: ConnectedSocket) -> Self::Socket {
        StreamSocket::new_priv(soc, self.protocol(), self.as_ctx()).into()
    }
}

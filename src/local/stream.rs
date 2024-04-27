use super::{LocalEndpoint, LocalProtocol};
use crate::ffi::{ConnectedSocket, IntoSocket};
use crate::listener::SocketListener;
use crate::socket_base::{AddressFamily, Protocol, SocketType};
use crate::stream::StreamSocket;

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

/// The stream-oriented UNIX domain socket type.
pub type LocalStreamSocket = StreamSocket<Stream>;

/// The stream-oriented UNIX domain listener type.
pub type LocalStreamListener = SocketListener<Stream, LocalStreamSocket>;

impl IntoSocket for LocalStreamListener {
    type Socket = LocalStreamSocket;

    fn into_socket(&self, soc: ConnectedSocket) -> Self::Socket {
        LocalStreamSocket::new_priv(self.as_ctx().clone(), self.protocol(), soc)
    }
}

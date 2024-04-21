use super::{LocalEndpoint, LocalProtocol};
use crate::listener::{ConnectedSocket, IntoConnectedSocket, SocketListener};
use crate::stream::StreamSocket;
use crate::{AddressFamily, Protocol, SocketType};

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

impl IntoConnectedSocket for LocalStreamListener {
    type Socket = LocalStreamSocket;

    fn into_connected_socket(&self, conn: ConnectedSocket) -> Self::Socket {
        LocalStreamSocket::new_priv(conn.ctx, self.protocol(), conn.soc)
    }
}

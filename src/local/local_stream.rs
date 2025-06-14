use super::{LocalEndpoint, LocalProtocol};
use crate::IoContext;
use crate::error::OsError;
use crate::socket::ffi::{self, Socket};
use crate::listener::{AsyncSocketListener, ConnectedSocket, SocketListener};
use crate::socket_base::{AddressFamily, Protocol, SocketType};
use crate::stream::{AsyncStreamSocket, StreamSocket};

/// The stream-oriented UNIX domain protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalStream;

impl LocalStream {
    pub fn connect(
        ctx: &IoContext,
        ep: &LocalEndpoint<Self>,
    ) -> Result<StreamSocket<Self>, OsError> {
        let soc = StreamSocket::new(ctx, Self)?;
        soc.connect(ep)
    }

    #[cfg(unix)]
    pub fn socketpair(
        ctx: &IoContext,
    ) -> Result<(StreamSocket<Self>, StreamSocket<Self>), OsError> {
        let (s1, s2) = ffi::socketpair(Self)?;
        Ok((
            StreamSocket::new_priv(ctx, s1, Self),
            StreamSocket::new_priv(ctx, s2, Self),
        ))
    }
}

impl Protocol for LocalStream {
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

impl ConnectedSocket for SocketListener<LocalStream> {
    type Socket = StreamSocket<LocalStream>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        Self::Socket::new_priv(self.as_ctx(), soc, self.protocol())
    }
}

impl ConnectedSocket for AsyncSocketListener<LocalStream> {
    type Socket = AsyncStreamSocket<LocalStream>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        StreamSocket::<LocalStream>::new_priv(self.as_ctx(), soc, self.protocol()).into()
    }
}

/// The stream-oriented UNIX domain endpoint type
pub type LocalStreamEndpoint = LocalEndpoint<LocalStream>;

/// The stream-oriented UNIX domain socket type
pub type LocalStreamSocket = StreamSocket<LocalStream>;

/// The stream-oriented UNIX domain listener type
pub type LocalStreamListener = SocketListener<LocalStream>;

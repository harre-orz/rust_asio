use super::{LocalEndpoint, LocalProtocol};
use crate::error::OsError;
use crate::ffi::{self, Socket};
use crate::listener::{AsyncSocketListener, ConnectedSocket, SocketListener};
use crate::socket_base::{AddressFamily, Protocol, SocketType};
use crate::stream::{AsyncStreamSocket, StreamSocket, StreamSocketBuilder};
use crate::IoContext;

/// The stream-oriented UNIX domain protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Stream;

impl Stream {
    pub fn connect<E>(ctx: &IoContext, ep: E) -> Result<StreamSocket<Self>, OsError>
    where
        E: AsRef<LocalEndpoint<Self>>,
    {
        StreamSocketBuilder::new(ctx, Self)?.connect(ep)
    }

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

impl ConnectedSocket for SocketListener<Stream> {
    type Socket = StreamSocket<Stream>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        Self::Socket::new_priv(self.as_ctx(), soc, self.protocol())
    }
}

impl ConnectedSocket for AsyncSocketListener<Stream> {
    type Socket = AsyncStreamSocket<Stream>;

    fn socket(&self, soc: Socket) -> Self::Socket {
        StreamSocket::new_priv(self.as_ctx(), soc, self.protocol()).into()
    }
}

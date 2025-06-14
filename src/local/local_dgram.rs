use super::{LocalEndpoint, LocalProtocol};
use crate::IoContext;
use crate::socket::ffi;
use crate::dgram::DgramSocket;
use crate::error::OsError;
use crate::socket_base::{AddressFamily, Protocol, SocketType};

/// The datagram-oriented UNIX domain protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct LocalDgram;

impl LocalDgram {
    pub fn connect(
        ctx: &IoContext,
        ep: &LocalEndpoint<Self>,
    ) -> Result<DgramSocket<Self>, OsError> {
        let soc = DgramSocket::new(ctx, Self)?;
        soc.connect(ep)?;
        Ok(soc)
    }

    #[cfg(unix)]
    pub fn socketpair(ctx: &IoContext) -> Result<(DgramSocket<Self>, DgramSocket<Self>), OsError> {
        let (s1, s2) = ffi::socketpair(Self)?;
        Ok((
            DgramSocket::new_priv(ctx, s1, Self),
            DgramSocket::new_priv(ctx, s2, Self),
        ))
    }
}

impl Protocol for LocalDgram {
    type Type = LocalProtocol;
    type Endpoint = LocalEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        AddressFamily::UNIX
    }

    fn socket_type(self) -> SocketType {
        SocketType::DGRAM
    }

    fn protocol_type(self) -> Self::Type {
        LocalProtocol
    }
}

/// The datagram-oriented UNIX domain endpoint type.
pub type LocalDgramEndpoint = LocalEndpoint<LocalDgram>;

/// The datagram-oriented UNIX domain socket type.
pub type LocalDgramSocket = DgramSocket<LocalDgram>;

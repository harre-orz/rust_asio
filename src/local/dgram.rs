use super::{LocalEndpoint, LocalProtocol};
use crate::dgram::{DgramSocket, DgramSocketBuilder};
use crate::error::OsError;
use crate::socket_base::{AddressFamily, Protocol, SocketType};
use crate::IoContext;

/// The datagram-oriented UNIX domain protocol.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Dgram;

impl Dgram {
    pub fn connect<E>(ctx: &IoContext, ep: E) -> Result<DgramSocket<Self>, OsError>
    where
        E: AsRef<LocalEndpoint<Self>>,
    {
        DgramSocketBuilder::new(ctx, Self)?.connect(ep)
    }
}

impl Protocol for Dgram {
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
pub type LocalDgramEndpoint = LocalEndpoint<Dgram>;

use super::{LocalEndpoint, LocalProtocol};
use crate::dgram::DgramSocket;
use crate::{AddressFamily, Protocol, SocketType};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Dgram;

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

pub type LocalDgramEndpoint = LocalEndpoint<Dgram>;
pub type LocalDgramSocket = DgramSocket<Dgram>;

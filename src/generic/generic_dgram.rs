use super::GenericEndpoint;
use crate::socket_base::{AddressFamily, IntoProtocolType, Protocol, SocketType};

pub struct GenericDgram<P>(AddressFamily, P);

impl<P: IntoProtocolType> Clone for GenericDgram<P> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<P: IntoProtocolType> Copy for GenericDgram<P> {}

impl<P: IntoProtocolType> GenericDgram<P> {
    pub const fn new(family: AddressFamily, protocol: P) -> Self {
        Self(family, protocol)
    }
}

impl<P> Protocol for GenericDgram<P>
where
    P: IntoProtocolType,
{
    type Type = P;
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::DGRAM
    }

    fn protocol_type(self) -> Self::Type {
        self.1
    }
}

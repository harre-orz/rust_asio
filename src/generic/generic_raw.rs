use super::GenericEndpoint;
use crate::socket_base::{AddressFamily, IntoProtocolType, Protocol, SocketType};

pub struct GenericRaw<P>(AddressFamily, P);

impl<P: IntoProtocolType> Clone for GenericRaw<P> {
    fn clone(&self) -> Self {
        Self(self.0, self.1)
    }
}

impl<P: IntoProtocolType> Copy for GenericRaw<P> {}

impl<P: IntoProtocolType> GenericRaw<P> {
    pub const fn new(family: AddressFamily, protocol: P) -> Self {
        Self(family, protocol)
    }
}

impl<P> Protocol for GenericRaw<P>
where
    P: IntoProtocolType,
{
    type Type = P;
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::RAW
    }

    fn protocol_type(self) -> Self::Type {
        self.1
    }
}

use crate::IoContext;
use crate::dgram_socket::{AsyncDgramSocket, DgramSocket, DgramSocketBuilder};
use crate::error::Result;
use crate::generic::GenericEndpoint;
use crate::sockaddr::AddressFamily;
use crate::socket_base::{EndpointRef, Protocol, SocketType};

#[derive(Copy, Clone)]
pub struct GenericRaw<T>(AddressFamily, T);

impl<T> Protocol for GenericRaw<T>
where
    T: Copy + Into<i32> + 'static,
{
    type Endpoint = GenericEndpoint<Self>;
    type Type = T;

    fn new(ep: &EndpointRef<Self::Endpoint>, protocol: Self::Type) -> Self {
        Self(AddressFamily::from_sockaddr(ep.sockaddr_ref()), protocol)
    }

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::SOCK_RAW
    }

    fn protocol_type(self) -> T {
        self.1
    }
}

impl<T> DgramSocket<GenericRaw<T>>
where
    T: Copy + Into<i32> + 'static,
{
    pub fn new(ctx: &IoContext, pro: T) -> DgramSocketBuilder<GenericRaw<T>> {
        DgramSocketBuilder::new_impl(ctx.clone(), pro)
    }
}

impl<T> DgramSocketBuilder<GenericRaw<T>>
where
    T: Copy + Into<i32> + 'static,
{
    pub fn unbound(self, address_family: AddressFamily) -> Result<DgramSocket<GenericRaw<T>>> {
        let pro = GenericRaw(address_family, self.protocol_type());
        self.unbound_impl(pro)
    }
}

pub type GenericRawEndpoint<T> = GenericEndpoint<GenericRaw<T>>;
pub type GenericRawSocket<T> = DgramSocket<GenericRaw<T>>;
pub type AsyncGenericRawSocket<T> = AsyncDgramSocket<GenericRaw<T>>;

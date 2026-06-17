use crate::IoContext;
use crate::dgram_socket::{AsyncDgramSocket, DgramSocket, DgramSocketBuilder};
use crate::error::OsError;
use crate::generic::GenericEndpoint;
use crate::sockaddr::AddressFamily;
use crate::socket_base::{EndpointRef, Protocol, SocketType};

#[derive(Copy, Clone)]
pub struct GenericDgram<T>(AddressFamily, T);

impl<T> Protocol for GenericDgram<T>
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
        SocketType::SOCK_DGRAM
    }

    fn protocol_type(self) -> T {
        self.1
    }
}

impl<T> DgramSocket<GenericDgram<T>>
where
    T: Copy + Into<i32> + 'static,
{
    pub fn new(ctx: &IoContext, pro: T) -> DgramSocketBuilder<GenericDgram<T>> {
        DgramSocketBuilder::new_impl(ctx.clone(), pro)
    }
}

impl<T> DgramSocketBuilder<GenericDgram<T>>
where
    T: Copy + Into<i32> + 'static,
{
    pub fn unbound(
        self,
        address_family: AddressFamily,
    ) -> Result<DgramSocket<GenericDgram<T>>, OsError> {
        let pro = GenericDgram(address_family, self.protocol_type());
        self.unbound_impl(pro)
    }
}

pub type GenericDgramEndpoint<T> = GenericEndpoint<GenericDgram<T>>;
pub type GenericDgramSocket<T> = DgramSocket<GenericDgram<T>>;
pub type AsyncGenericDgramSocket<T> = AsyncDgramSocket<GenericDgram<T>>;

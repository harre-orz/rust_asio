use crate::IoContext;
use crate::dgram_socket::{AsyncDgramSocket, DgramSocket, DgramSocketBuilder};
use crate::error::OsError;
use crate::generic::GenericEndpoint;
use crate::sockaddr::{AddressFamily, SockAddr};
use crate::socket::{Socket, SocketType};
use crate::socket_base::Protocol;
use std::result;

type Result<T> = result::Result<T, OsError>;

#[derive(Copy, Clone)]
pub struct GenericRaw<T>(AddressFamily, T);

impl<T> GenericRaw<T> {
    pub const fn new(family: AddressFamily, protocol: T) -> Self {
        Self(family, protocol)
    }
}

impl<T> Protocol for GenericRaw<T>
where
    T: Copy + Into<i32>,
{
    type Type = T;
    type Endpoint = GenericEndpoint<Self>;

    fn new(address_family: AddressFamily, protocol: Self::Type) -> Self {
        Self(address_family, protocol)
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

impl<T> GenericEndpoint<GenericRaw<T>>
where
    T: Copy + Into<i32>,
{
    pub fn protocol(&self, pro: T) -> GenericRaw<T> {
        GenericRaw(self.ss.address_family(), pro)
    }
}

impl<T> DgramSocket<GenericRaw<T>>
where
    T: Copy + Into<i32>,
{
    pub fn new(ctx: &IoContext, pro: T) -> DgramSocketBuilder<GenericRaw<T>> {
        DgramSocketBuilder::new_impl(ctx.clone(), pro)
    }
}

impl<T> DgramSocketBuilder<GenericRaw<T>>
where
    T: Copy + Into<i32>,
{
    pub fn unbound(self, address_family: AddressFamily) -> Result<DgramSocket<GenericRaw<T>>> {
        let pro = GenericRaw::new(address_family, self.pro);
        let soc = Socket::new(pro)?;
        Ok(DgramSocket::new_impl(self.ctx, soc))
    }
}

pub type GenericRawEndpoint<T> = GenericEndpoint<GenericRaw<T>>;
pub type GenericRawSocket<T> = DgramSocket<GenericRaw<T>>;
pub type AsyncGenericRawSocket<T> = AsyncDgramSocket<GenericRaw<T>>;

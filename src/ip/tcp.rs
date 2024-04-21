use super::{IpEndpoint, IpProtocol, Resolver};
use crate::listener::{ConnectedSocket, IntoConnectedSocket, SocketListener};
use crate::stream::StreamSocket;
use crate::{AddressFamily, IoContext, Protocol, SocketType};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct Tcp(AddressFamily);

impl Tcp {
    pub const V4: Self = Self(AddressFamily::INET);
    pub const V6: Self = Self(AddressFamily::INET6);
}

impl Protocol for Tcp {
    type Type = IpProtocol;
    type Endpoint = IpEndpoint<Self>;

    fn family_type(self) -> AddressFamily {
        self.0
    }

    fn socket_type(self) -> SocketType {
        SocketType::STREAM
    }

    fn protocol_type(self) -> IpProtocol {
        IpProtocol::TCP
    }
}

pub type TcpEndpoint = IpEndpoint<Tcp>;
pub type TcpSocket = StreamSocket<Tcp>;
pub type TcpListener = SocketListener<Tcp, TcpSocket>;
pub type TcpResolver = Resolver<Tcp>;

impl TcpResolver {
    pub fn new(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Tcp(AddressFamily::UNSPEC))
    }

    pub fn v4(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Tcp::V4)
    }

    pub fn v6(ctx: &IoContext) -> Self {
        Self::new_priv(ctx, Tcp::V6)
    }
}

impl IntoConnectedSocket for TcpListener {
    type Socket = TcpSocket;

    fn into_connected_socket(&self, soc: ConnectedSocket) -> Self::Socket {
        TcpSocket::new_priv(self.as_ctx().clone(), self.protocol(), soc.0)
    }
}

#[test]
fn test_resolve() {
    use crate::ip::*;

    let ctx = &IoContext::new();


    let mut it: ResolverIter<Tcp> = TcpResolver::v4(ctx).resolve(("127.0.0.1", 80)).unwrap();
    let ep = it.next().unwrap();

    println!("{:?}", ep);
}

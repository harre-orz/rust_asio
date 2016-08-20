use {Protocol, SockAddr, Endpoint, StreamSocket, SocketListener};
use backbone::SOCK_STREAM;
use super::GenericEndpoint;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct GenericStream {
    family: i32,
    protocol: i32,
    capacity: usize,
}

impl Protocol for GenericStream {
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_STREAM
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        GenericEndpoint::default(self.capacity, self.protocol)
    }
}

impl Endpoint<GenericStream> for GenericEndpoint<GenericStream> {
    fn protocol(&self) -> GenericStream {
        GenericStream {
            family: self.as_sockaddr().sa_family as i32,
            protocol: self.protocol,
            capacity: self.capacity(),
        }
    }
}

pub type GenericStreamEndpoint = GenericEndpoint<GenericStream>;

pub type GenericStreamSocket = StreamSocket<GenericStream>;

pub type GenericStreamListener = SocketListener<GenericStream>;

#[test]
fn test_generic_tcp() {
    use IoService;
    use ip::{IpAddrV4, TcpEndpoint};
    use socket_base::ReuseAddr;

    let io = &IoService::new();
    let ep = GenericStreamEndpoint::new(&TcpEndpoint::new(IpAddrV4::any(), 12345), 0);
    let soc = GenericStreamListener::new(io, ep.protocol()).unwrap();
    soc.set_option(ReuseAddr::new(true)).unwrap();
    soc.bind(&ep).unwrap();
    soc.listen().unwrap();
}

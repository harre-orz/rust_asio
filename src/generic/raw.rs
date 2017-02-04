use prelude::{Protocol, SockAddr, Endpoint};
use ffi::SOCK_RAW;
use raw_socket::RawSocket;
use generic::GenericEndpoint;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct GenericRaw {
    family: i32,
    protocol: i32,
    capacity: usize,
}

impl Protocol for GenericRaw {
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_RAW
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        GenericEndpoint::default(self.capacity, self.protocol)
    }
}

impl Endpoint<GenericRaw> for GenericEndpoint<GenericRaw> {
    fn protocol(&self) -> GenericRaw {
        GenericRaw {
            family: self.as_ref().sa_family as i32,
            protocol: self.protocol,
            capacity: self.capacity(),
        }
    }
}

pub type GenericRawEndpoint = GenericEndpoint<GenericRaw>;

pub type GenericRawSocket = RawSocket<GenericRaw>;

use traits::{Protocol, SockAddr, Endpoint};
use dgram_socket::DgramSocket;
use libc::SOCK_DGRAM;
use super::GenericEndpoint;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct GenericDgram {
    family: i32,
    protocol: i32,
    capacity: usize,
}

impl Protocol for GenericDgram {
    type Endpoint = GenericEndpoint<Self>;

    fn family_type(&self) -> i32 {
        self.family
    }

    fn socket_type(&self) -> i32 {
        SOCK_DGRAM
    }

    fn protocol_type(&self) -> i32 {
        self.protocol
    }

    unsafe fn uninitialized(&self) -> Self::Endpoint {
        GenericEndpoint::default(self.capacity, self.protocol)
    }
}

impl Endpoint<GenericDgram> for GenericEndpoint<GenericDgram> {
    fn protocol(&self) -> GenericDgram {
        GenericDgram {
            family: self.as_sockaddr().sa_family as i32,
            protocol: self.protocol,
            capacity: self.capacity(),
        }
    }
}

pub type GenericDgramEndpoint = GenericEndpoint<GenericDgram>;

pub type GenericDgramSocket = DgramSocket<GenericDgram>;

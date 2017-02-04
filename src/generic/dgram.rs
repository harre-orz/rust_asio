use prelude::{Protocol, SockAddr, Endpoint};
use ffi::SOCK_DGRAM;
use dgram_socket::DgramSocket;
use generic::GenericEndpoint;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
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
            family: self.as_ref().sa_family as i32,
            protocol: self.protocol,
            capacity: self.capacity(),
        }
    }
}

pub type GenericDgramEndpoint = GenericEndpoint<GenericDgram>;

pub type GenericDgramSocket = DgramSocket<GenericDgram>;
